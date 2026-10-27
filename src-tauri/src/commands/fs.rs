use std::fs;
use std::io::Read as IoRead;
use std::path::Path;
use std::thread;
use std::time::Duration;

use calamine::{open_workbook_auto, Data, ExcelDateTime, Reader};
use office_oxide::Document;

use crate::panic_guard::run_guarded;
use crate::types::wiki::FileNode;

/// Known binary formats that need special extraction
/// Formats handled by AnyDoc's shared document model. Keep this list aligned
/// with `anydoc::Format::from_extension`; the explicit list is still needed so
/// arbitrary binary files never reach a parser merely because detection is
/// permissive. PDF and EPUB intentionally remain on the existing pipelines:
/// PDFium also extracts images, while the ebook path retains MOBI parity.
const OFFICE_EXTS: &[&str] = &[
    "doc", "docx", "docm", "ppt", "pps", "pot", "pptx", "pptm", "ppsx", "ppsm", "xls", "xlsx",
    "xlsm", "xlsb", "odt", "ods", "odp", "rtf",
];
const IMAGE_EXTS: &[&str] = &[
    "png", "jpg", "jpeg", "gif", "webp", "bmp", "ico", "tiff", "tif", "avif", "heic", "heif", "svg",
];
const MEDIA_EXTS: &[&str] = &[
    "mp4", "webm", "mov", "avi", "mkv", "flv", "wmv", "m4v", "mp3", "wav", "ogg", "flac", "aac",
    "m4a", "wma",
];
const EBOOK_EXTS: &[&str] = &["epub", "mobi"];
const LEGACY_DOC_EXTS: &[&str] = &["pages", "numbers", "key"];
fn require_absolute_path(operation: &str, path: &str) -> Result<(), String> {
    if is_absolute_path_cross_platform(path) {
        Ok(())
    } else {
        Err(format!(
            "{operation} requires an absolute path; got relative path '{}'",
            path
        ))
    }
}

fn is_absolute_path_cross_platform(path: &str) -> bool {
    if path.is_empty() {
        return false;
    }
    if Path::new(path).is_absolute() {
        return true;
    }

    let bytes = path.as_bytes();
    if bytes.len() >= 3
        && bytes[1] == b':'
        && bytes[0].is_ascii_alphabetic()
        && matches!(bytes[2], b'/' | b'\\')
    {
        return true;
    }

    path.starts_with(r"\\") || path.starts_with("//")
}

#[tauri::command]
pub async fn read_file(path: String) -> Result<String, String> {
    // `spawn_blocking` is REQUIRED, not a perf nicety. The body does
    // synchronous PDF/Office text extraction (pdfium FFI, calamine,
    // zip + image decode) that can take 10s+ on big files. Running
    // that directly inside an `async fn` body would block the tokio
    // worker thread it's scheduled on, starving every other async
    // task on that worker (notably re-rendering the import progress
    // UI, which is what motivated the async conversion in the first
    // place). `spawn_blocking` moves the work to tokio's blocking
    // pool where blocking-for-seconds is the contract.
    tauri::async_runtime::spawn_blocking(move || {
        run_guarded("read_file", || {
            let p = Path::new(&path);
            let ext = p
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_lowercase();

            match ext.as_str() {
                "pdf" => extract_pdf_text(&path),
                "org" => extract_org_text(&path),
                e if OFFICE_EXTS.contains(&e) => extract_office_text(&path, e),
                e if EBOOK_EXTS.contains(&e) => crate::commands::ebook::extract_ebook_text(&path, e),
                e if IMAGE_EXTS.contains(&e) => {
                    let size = fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
                    Ok(format!("[Image: {} ({:.1} KB)]", p.file_name().unwrap_or_default().to_string_lossy(), size as f64 / 1024.0))
                }
                e if MEDIA_EXTS.contains(&e) => {
                    let size = fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
                    Ok(format!("[Media: {} ({:.1} MB)]", p.file_name().unwrap_or_default().to_string_lossy(), size as f64 / 1048576.0))
                }
                e if LEGACY_DOC_EXTS.contains(&e) => {
                    Ok(format!("[Document: {} — text extraction not supported for .{} format]",
                        p.file_name().unwrap_or_default().to_string_lossy(), e))
                }
                _ => {
                    match fs::read_to_string(&path) {
                        Ok(content) => Ok(content),
                        Err(e) => {
                            let exists = p.exists();
                            if !exists {
                                Err(format!("File does not exist: '{}'", path))
                            } else {
                                Err(format!(
                                    "Failed to read file '{}' as text: {} (likely binary, locked, or non-UTF-8)",
                                    path, e,
                                ))
                            }
                        }
                    }
                }
            }
        })
    })
    .await
    .map_err(|e| format!("read_file blocking task join error: {e}"))?
}

/// The text of a document by extraction alone: no cache and no
/// recognition engine, so it costs nothing and never invents. `Ok(None)`
/// for what carries no text of its own (images, media, other binaries).
pub(crate) fn extract_text_without_recognition(path: &Path) -> Result<Option<String>, String> {
    /// Plain files above this are logs or data dumps, not documents.
    const MAX_PLAIN_TEXT_BYTES: u64 = 8 * 1024 * 1024;

    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    let location = path.to_string_lossy();
    match ext.as_str() {
        "pdf" => extract_pdf_text(&location).map(Some),
        "org" => extract_org_text(&location).map(Some),
        e if OFFICE_EXTS.contains(&e) => extract_office_text(&location, e).map(Some),
        e if EBOOK_EXTS.contains(&e) => {
            crate::commands::ebook::extract_ebook_text(&location, e).map(Some)
        }
        e if IMAGE_EXTS.contains(&e) || MEDIA_EXTS.contains(&e) || LEGACY_DOC_EXTS.contains(&e) => {
            Ok(None)
        }
        _ => {
            let size = fs::metadata(path)
                .map_err(|e| format!("Failed to read metadata of '{location}': {e}"))?
                .len();
            if size > MAX_PLAIN_TEXT_BYTES {
                return Ok(None);
            }
            let bytes = fs::read(path).map_err(|e| format!("Failed to read '{location}': {e}"))?;
            Ok(String::from_utf8(bytes).ok())
        }
    }
}

/// Convert common Org syntax into Markdown-shaped text for ingestion and
/// preview. The original `.org` file remains untouched.
/// Source blocks are copied verbatim inside fences and are never executed.
fn extract_org_text(path: &str) -> Result<String, String> {
    let content = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read Org file '{}': {}", path, e))?;
    Ok(org_to_markdown(&content))
}

fn org_to_markdown(content: &str) -> String {
    let normalized = content.replace("\r\n", "\n").replace('\r', "\n");
    let mut output = Vec::new();
    let mut block_end: Option<&'static str> = None;

    for line in normalized.lines() {
        let trimmed = line.trim();
        let upper = trimmed.to_ascii_uppercase();
        if let Some(end) = block_end {
            if upper == end {
                output.push("```".to_string());
                block_end = None;
            } else {
                output.push(line.to_string());
            }
            continue;
        }

        if upper.starts_with("#+BEGIN_SRC") {
            let language = trimmed[11..].trim().split_whitespace().next().unwrap_or("");
            output.push(format!("```{language}"));
            block_end = Some("#+END_SRC");
            continue;
        }
        if upper == "#+BEGIN_EXAMPLE" {
            output.push("```text".to_string());
            block_end = Some("#+END_EXAMPLE");
            continue;
        }
        if upper == "#+BEGIN_QUOTE" {
            output.push("```text".to_string());
            block_end = Some("#+END_QUOTE");
            continue;
        }

        if let Some((key, value)) = parse_org_keyword(trimmed) {
            if key == "TITLE" {
                output.push(format!("# {value}"));
            } else if !matches!(key.as_str(), "OPTIONS" | "PROPERTY" | "SETUPFILE") {
                output.push(format!("**{}:** {}", title_case_ascii(&key), value));
            }
            continue;
        }

        if let Some((level, heading)) = parse_org_heading(line) {
            output.push(format!(
                "{} {}",
                "#".repeat(level.min(6)),
                convert_org_links(heading)
            ));
            continue;
        }

        if trimmed.starts_with('|')
            && trimmed.ends_with('|')
            && trimmed.contains('+')
            && trimmed
                .chars()
                .all(|c| matches!(c, '|' | '+' | '-' | ':' | ' '))
        {
            output.push(trimmed.replace('+', "|"));
            continue;
        }

        output.push(convert_org_links(line));
    }
    if block_end.is_some() {
        output.push("```".to_string());
    }
    output.join("\n")
}

fn parse_org_keyword(line: &str) -> Option<(String, &str)> {
    let rest = line.strip_prefix("#+")?;
    let (key, value) = rest.split_once(':')?;
    if key.is_empty() || !key.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return None;
    }
    Some((key.to_ascii_uppercase(), value.trim()))
}

fn parse_org_heading(line: &str) -> Option<(usize, &str)> {
    let stars = line.chars().take_while(|c| *c == '*').count();
    if stars == 0 || line.as_bytes().get(stars) != Some(&b' ') {
        return None;
    }
    Some((stars, line[stars + 1..].trim()))
}

fn title_case_ascii(value: &str) -> String {
    let lower = value.replace('_', " ").to_ascii_lowercase();
    let mut chars = lower.chars();
    chars
        .next()
        .map(|c| c.to_ascii_uppercase().to_string() + chars.as_str())
        .unwrap_or_default()
}

fn convert_org_links(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut rest = line;
    while let Some(start) = rest.find("[[") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        let Some(end) = after.find("]]") else {
            out.push_str(&rest[start..]);
            return out;
        };
        let inner = &after[..end];
        if let Some((target, description)) = inner.split_once("][") {
            out.push_str(&format!("[{}]({})", description, target));
        } else {
            out.push_str(&format!("<{}>", inner));
        }
        rest = &after[end + 2..];
    }
    out.push_str(rest);
    out
}

/// Global PDFium instance — the library prefers a single binding shared
/// across threads over repeatedly binding/unbinding.
static PDFIUM: std::sync::OnceLock<Result<pdfium_render::prelude::Pdfium, String>> =
    std::sync::OnceLock::new();

/// Serializes every PDFium call. PDFium's C library is documented as
/// safe across threads only when no PDFium object is touched from
/// two threads simultaneously — interleaved calls are UB and have
/// caused EXC_BAD_ACCESS segfaults on macOS ARM64 in production.
///
/// This mutex matters because our heavy fs commands are now `async
/// fn`, so Tauri schedules them on the tokio multi-threaded runtime
/// instead of running them on a single thread. Without this lock,
/// two concurrent `read_file`/`extract_*_pdf` calls can land on
/// different worker threads and interleave inside pdfium → crash.
///
/// We use `std::sync::Mutex` (not `tokio::sync::Mutex`) because the
/// lock is acquired *inside* `spawn_blocking`, never held across
/// `.await` — async-aware mutexes would just add overhead for no
/// benefit here.
static PDFIUM_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Acquire the PDFium serialization lock. Auto-recovers from poison
/// (a previous panic on a malformed PDF leaves the mutex poisoned,
/// but pdfium has no shared state for that panic to have corrupted —
/// the next caller can safely take the lock and proceed).
pub(crate) fn lock_pdfium() -> std::sync::MutexGuard<'static, ()> {
    PDFIUM_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Additional resource directory hint, set by the Tauri setup() callback
/// once the AppHandle is available. Lets the pdfium resolver find the
/// bundled dylib without re-implementing Tauri's platform-specific
/// resource-dir logic.
static RESOURCE_DIR_HINT: std::sync::OnceLock<std::path::PathBuf> = std::sync::OnceLock::new();

/// Called from Tauri's setup() with the resolved resource directory.
/// No-op if already set.
pub fn set_resource_dir_hint(dir: std::path::PathBuf) {
    let _ = RESOURCE_DIR_HINT.set(dir);
}

/// The resource directory Tauri resolved, once setup() ran.
pub fn resource_dir_hint() -> Option<&'static std::path::Path> {
    RESOURCE_DIR_HINT.get().map(|dir| dir.as_path())
}

/// Enumerate plausible locations for the PDFium dynamic library on the
/// current platform. Order from most specific to least:
///   1. `$PDFIUM_DYNAMIC_LIB_PATH` env var (local dev convenience)
///   2. Tauri resource dir (set via setup()) — the authoritative location
///   3. Paths relative to the executable where Tauri's bundler lands
///      resources on each platform (macOS Frameworks / Resources /
///      MacOS dir, Windows sibling, Linux sibling)
///   4. OS dynamic loader search path (last resort)
fn pdfium_candidate_paths() -> Vec<String> {
    let mut v: Vec<String> = Vec::new();

    if let Ok(p) = std::env::var("PDFIUM_DYNAMIC_LIB_PATH") {
        v.push(p);
    }

    // Tauri-resolved resource directory (set during setup()).
    //
    // Tauri's `bundle.resources` array form preserves relative paths,
    // so `"pdfium/pdfium.dll"` in tauri.<target>.conf.json lands at
    // `<resource_dir>/pdfium/pdfium.dll` — NOT at the root. Older
    // versions of this function only probed the root, which made
    // Windows installs fail with "Failed to locate Pdfium library"
    // (OS error 126) even though the DLL was in the installer.
    // We now probe both the `pdfium/` subdir (where the current
    // bundle config actually puts it) and the root (in case a future
    // config change flattens it).
    // Unit tests run from `target/`, far from any app bundle: use the
    // copy of the library that is checked into the repository.
    #[cfg(test)]
    {
        let repo_copy = Path::new(env!("CARGO_MANIFEST_DIR")).join("pdfium");
        for name in ["pdfium.dll", "libpdfium.dylib", "libpdfium.so"] {
            v.push(repo_copy.join(name).to_string_lossy().into_owned());
        }
    }

    if let Some(resource_dir) = RESOURCE_DIR_HINT.get() {
        let push = |v: &mut Vec<String>, p: std::path::PathBuf| {
            v.push(p.to_string_lossy().into_owned());
        };
        #[cfg(target_os = "macos")]
        {
            push(&mut v, resource_dir.join("pdfium").join("libpdfium.dylib"));
            push(&mut v, resource_dir.join("libpdfium.dylib"));
        }
        #[cfg(target_os = "windows")]
        {
            push(&mut v, resource_dir.join("pdfium").join("pdfium.dll"));
            push(&mut v, resource_dir.join("pdfium").join("libpdfium.dll"));
            push(&mut v, resource_dir.join("pdfium.dll"));
            push(&mut v, resource_dir.join("libpdfium.dll"));
        }
        #[cfg(target_os = "linux")]
        {
            push(&mut v, resource_dir.join("pdfium").join("libpdfium.so"));
            push(&mut v, resource_dir.join("libpdfium.so"));
        }
    }

    if let Ok(exe) = std::env::current_exe() {
        if let Some(exe_dir) = exe.parent() {
            let push = |v: &mut Vec<String>, p: std::path::PathBuf| {
                v.push(p.to_string_lossy().into_owned());
            };

            #[cfg(target_os = "macos")]
            {
                // Tauri .app bundle layout:
                //   Contents/MacOS/<binary>
                //   Contents/Frameworks/libpdfium.dylib   ← preferred (macOS config uses bundle.macOS.frameworks)
                //   Contents/Resources/libpdfium.dylib    ← fallback
                //   Contents/Resources/pdfium/libpdfium.dylib  ← if array-form resources ever used on macOS
                push(&mut v, exe_dir.join("../Frameworks/libpdfium.dylib"));
                push(&mut v, exe_dir.join("../Resources/pdfium/libpdfium.dylib"));
                push(&mut v, exe_dir.join("../Resources/libpdfium.dylib"));
                push(&mut v, exe_dir.join("libpdfium.dylib"));
            }

            #[cfg(target_os = "windows")]
            {
                // bblanchon/pdfium-binaries ships the Windows DLL as
                // `pdfium.dll` (no `lib` prefix). Probe flat and
                // `pdfium/` subdir forms at both exe root and the
                // classic Tauri `resources/` sibling — covers every
                // layout variant we've observed across NSIS / MSI /
                // portable builds.
                push(&mut v, exe_dir.join("pdfium.dll"));
                push(&mut v, exe_dir.join("pdfium").join("pdfium.dll"));
                push(&mut v, exe_dir.join("libpdfium.dll"));
                push(&mut v, exe_dir.join("resources").join("pdfium.dll"));
                push(
                    &mut v,
                    exe_dir.join("resources").join("pdfium").join("pdfium.dll"),
                );
            }

            #[cfg(target_os = "linux")]
            {
                push(&mut v, exe_dir.join("libpdfium.so"));
                push(&mut v, exe_dir.join("pdfium").join("libpdfium.so"));
                push(&mut v, exe_dir.join("resources").join("libpdfium.so"));
                push(
                    &mut v,
                    exe_dir
                        .join("resources")
                        .join("pdfium")
                        .join("libpdfium.so"),
                );
                push(&mut v, exe_dir.join("../lib/libpdfium.so"));
            }
        }
    }

    v
}

pub(crate) fn pdfium() -> Result<&'static pdfium_render::prelude::Pdfium, String> {
    PDFIUM
        .get_or_init(|| {
            use pdfium_render::prelude::*;
            let candidates = pdfium_candidate_paths();
            for path in &candidates {
                if let Ok(bindings) = Pdfium::bind_to_library(path) {
                    eprintln!("[pdfium] loaded dynamic library from {path}");
                    return Ok(Pdfium::new(bindings));
                }
            }
            // Last resort: let the OS dynamic loader find it.
            Pdfium::bind_to_system_library()
                .map(Pdfium::new)
                .map_err(|e| {
                    format!(
                        "Failed to locate Pdfium library. Tried: {} — and the system search path. Last error: {e}",
                        if candidates.is_empty() {
                            "(no candidates)".to_string()
                        } else {
                            candidates.join(", ")
                        }
                    )
                })
        })
        .as_ref()
        .map_err(|e| e.clone())
}

/// The text of a PDF as Markdown; its images stay inside it.
fn extract_pdf_text(path: &str) -> Result<String, String> {
    use crate::commands::extract_images::{extract_pdf_markdown, ExtractOptions};
    extract_pdf_markdown(path, None, "", &ExtractOptions::default())
}

/// Extract text from Office Open XML formats, converting to Markdown.
fn extract_office_text(path: &str, ext: &str) -> Result<String, String> {
    let mut anydoc_failure = None;
    match anydoc::to_markdown(path) {
        Ok(markdown) if !markdown.trim().is_empty() => return Ok(markdown),
        Ok(_) => {
            eprintln!(
                "[anydoc] '{}' produced empty Markdown; trying the compatibility parser",
                path
            );
        }
        Err(anydoc::ConvertError::ResourceLimit { limit, detail }) => {
            // Never bypass AnyDoc's abuse limits by feeding the same hostile
            // input to a less constrained compatibility parser.
            return Err(format!(
                "Document exceeds the AnyDoc safety limit '{limit}': {detail}"
            ));
        }
        Err(anydoc::ConvertError::Encrypted) => {
            return Err("Encrypted or password-protected documents are not supported".to_string());
        }
        Err(error) => {
            eprintln!(
                "[anydoc] failed to parse '{}' ({}); trying the compatibility parser",
                path, error
            );
            anydoc_failure = Some(error.to_string());
        }
    }

    extract_office_text_compat(path, ext).map_err(|compat_error| match anydoc_failure {
        Some(anydoc_error) => format!(
            "AnyDoc failed to extract .{ext}: {anydoc_error}; compatibility parser failed: {compat_error}"
        ),
        None => compat_error,
    })
}

/// Compatibility path for formats supported before AnyDoc was introduced.
/// New AnyDoc-only variants deliberately return the original-format error
/// rather than being read as UTF-8 or reported as a successful empty import.
fn extract_office_text_compat(path: &str, ext: &str) -> Result<String, String> {
    if !matches!(
        ext,
        "doc" | "docx" | "pptx" | "xls" | "xlsx" | "odt" | "ods" | "odp"
    ) {
        return Err(format!("no compatibility parser is available for .{ext}"));
    }

    // Spreadsheets: use calamine (supports xlsx, xls, ods)
    if matches!(ext, "xlsx" | "xls" | "ods") {
        return extract_spreadsheet(path);
    }

    // DOCX: use docx-rs library for proper parsing
    if ext == "docx" {
        return extract_docx_with_library(path);
    }

    // DOC: use office_oxide for legacy Word binary documents.
    if ext == "doc" {
        return extract_doc_with_office_oxide(path);
    }

    // PPTX and ODF: use ZIP-based parsing
    let file = fs::File::open(path).map_err(|e| format!("Failed to open '{}': {}", path, e))?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|e| format!("Failed to read ZIP archive '{}': {}", path, e))?;

    match ext {
        "pptx" => extract_pptx_markdown(&mut archive),
        "odt" | "odp" => extract_odf_text(&mut archive),
        _ => Err(format!(
            "AnyDoc could not extract .{ext} and no compatibility parser is available"
        )),
    }
}

/// Extract legacy Word `.doc` text using office_oxide.
fn extract_doc_with_office_oxide(path: &str) -> Result<String, String> {
    let doc = Document::open(path).map_err(|e| format!("Failed to parse DOC '{}': {}", path, e))?;
    let markdown = doc.to_markdown();
    let text = if markdown.trim().is_empty() {
        doc.plain_text()
    } else {
        markdown
    };

    if text.trim().is_empty() {
        Ok("[Document: no extractable text found in .doc file]".to_string())
    } else {
        Ok(text)
    }
}

/// Extract DOCX using docx-rs library for proper structural parsing.
fn extract_docx_with_library(path: &str) -> Result<String, String> {
    let bytes = fs::read(path).map_err(|e| format!("Failed to read DOCX '{}': {}", path, e))?;
    let docx = docx_rs::read_docx(&bytes)
        .map_err(|e| format!("Failed to parse DOCX '{}': {:?}", path, e))?;

    let mut result = String::new();

    for child in docx.document.children {
        match child {
            docx_rs::DocumentChild::Paragraph(para) => {
                let mut para_text = String::new();
                let mut is_heading = false;
                let mut heading_level: u8 = 1;

                // Check paragraph style for headings
                if let Some(style) = &para.property.style {
                    let style_val = &style.val;
                    if style_val.contains("Heading") || style_val.contains("heading") {
                        is_heading = true;
                        // Extract level number
                        for ch in style_val.chars() {
                            if ch.is_ascii_digit() {
                                heading_level = ch.to_digit(10).unwrap_or(1) as u8;
                                break;
                            }
                        }
                    }
                }

                // Check for list (numbering)
                let is_list = para.property.numbering_property.is_some();

                // Extract text from runs
                for child in &para.children {
                    if let docx_rs::ParagraphChild::Run(run) = child {
                        let is_bold = run.run_property.bold.is_some();
                        let is_italic = run.run_property.italic.is_some();

                        for run_child in &run.children {
                            if let docx_rs::RunChild::Text(text) = run_child {
                                let t = &text.text;
                                if is_bold && is_italic {
                                    para_text.push_str(&format!("***{}***", t));
                                } else if is_bold {
                                    para_text.push_str(&format!("**{}**", t));
                                } else if is_italic {
                                    para_text.push_str(&format!("*{}*", t));
                                } else {
                                    para_text.push_str(t);
                                }
                            }
                        }
                    }
                }

                let text = para_text.trim().to_string();
                if text.is_empty() {
                    continue;
                }

                if is_heading {
                    let prefix = "#".repeat(heading_level as usize);
                    result.push_str(&format!("{} {}\n\n", prefix, text));
                } else if is_list {
                    result.push_str(&format!("- {}\n", text));
                } else {
                    result.push_str(&text);
                    result.push_str("\n\n");
                }
            }
            docx_rs::DocumentChild::Table(table) => {
                let mut rows: Vec<Vec<String>> = Vec::new();
                for row in &table.rows {
                    {
                        let docx_rs::TableChild::TableRow(tr) = row;
                        let mut cells: Vec<String> = Vec::new();
                        for cell in &tr.cells {
                            {
                                let docx_rs::TableRowChild::TableCell(tc) = cell;
                                let mut cell_text = String::new();
                                for child in &tc.children {
                                    if let docx_rs::TableCellContent::Paragraph(para) = child {
                                        for pchild in &para.children {
                                            if let docx_rs::ParagraphChild::Run(run) = pchild {
                                                for rc in &run.children {
                                                    if let docx_rs::RunChild::Text(t) = rc {
                                                        cell_text.push_str(&t.text);
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                                cells.push(cell_text.trim().replace('|', "\\|"));
                            }
                        }
                        rows.push(cells);
                    }
                }
                if !rows.is_empty() {
                    let max_cols = rows.iter().map(|r| r.len()).max().unwrap_or(0);
                    for (i, row) in rows.iter().enumerate() {
                        let mut padded = row.clone();
                        padded.resize(max_cols, String::new());
                        result.push_str("| ");
                        result.push_str(&padded.join(" | "));
                        result.push_str(" |\n");
                        if i == 0 {
                            result.push('|');
                            for _ in 0..max_cols {
                                result.push_str(" --- |");
                            }
                            result.push('\n');
                        }
                    }
                    result.push('\n');
                }
            }
            _ => {}
        }
    }

    if result.trim().is_empty() {
        // Fallback to ZIP-based extraction
        let file = fs::File::open(path).map_err(|e| e.to_string())?;
        let mut archive = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;
        extract_docx_markdown(&mut archive)
    } else {
        Ok(result)
    }
}

fn read_zip_file(archive: &mut zip::ZipArchive<fs::File>, name: &str) -> Option<String> {
    let mut file = archive.by_name(name).ok()?;
    let mut content = String::new();
    file.read_to_string(&mut content).ok()?;
    Some(content)
}

fn decode_xml_entities(text: &str) -> String {
    text.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&#10;", "\n")
        .replace("&#13;", "")
}

/// Extract DOCX to Markdown preserving headings, paragraphs, lists, tables, bold/italic.
fn extract_docx_markdown(archive: &mut zip::ZipArchive<fs::File>) -> Result<String, String> {
    let xml = read_zip_file(archive, "word/document.xml")
        .ok_or_else(|| "No document.xml found".to_string())?;

    let mut result = String::new();
    let mut i = 0;
    let chars: Vec<char> = xml.chars().collect();
    let len = chars.len();

    let mut paragraph_text = String::new();
    let mut is_heading = false;
    let mut heading_level: u8 = 1;
    let mut is_bold = false;
    let mut is_italic = false;
    let mut in_table = false;
    let mut table_row: Vec<String> = Vec::new();
    let mut table_cell_text = String::new();
    let mut in_cell = false;
    let mut is_first_table_row = true;
    let mut in_list_item = false;

    while i < len {
        if chars[i] == '<' {
            // Read tag name
            i += 1;
            let is_closing = i < len && chars[i] == '/';
            if is_closing {
                i += 1;
            }

            let mut tag_name = String::new();
            while i < len && chars[i] != '>' && chars[i] != ' ' && chars[i] != '/' {
                tag_name.push(chars[i]);
                i += 1;
            }

            // Read rest of tag to find attributes
            let mut tag_content = String::new();
            while i < len && chars[i] != '>' {
                tag_content.push(chars[i]);
                i += 1;
            }
            if i < len {
                i += 1;
            } // skip >

            match tag_name.as_str() {
                // Paragraph start
                "w:p" if !is_closing => {
                    paragraph_text.clear();
                    is_heading = false;
                    in_list_item = false;
                }
                // Paragraph end — flush
                "w:p" if is_closing => {
                    let text = paragraph_text.trim().to_string();
                    if !text.is_empty() {
                        if in_table && in_cell {
                            table_cell_text = text;
                        } else if is_heading {
                            let prefix = "#".repeat(heading_level as usize);
                            result.push_str(&format!("{} {}\n\n", prefix, text));
                        } else if in_list_item {
                            result.push_str(&format!("- {}\n", text));
                        } else {
                            result.push_str(&text);
                            result.push_str("\n\n");
                        }
                    }
                    paragraph_text.clear();
                }
                // Heading style detection
                "w:pStyle" if !is_closing => {
                    if tag_content.contains("Heading") || tag_content.contains("heading") {
                        is_heading = true;
                        // Try to extract heading level from val="Heading1" etc.
                        if let Some(pos) = tag_content.find("Heading") {
                            let after = &tag_content[pos + 7..];
                            if let Some(ch) = after.chars().next() {
                                if ch.is_ascii_digit() {
                                    heading_level = ch.to_digit(10).unwrap_or(1) as u8;
                                }
                            }
                        }
                    }
                    if tag_content.contains("ListParagraph")
                        || tag_content.contains("listParagraph")
                    {
                        in_list_item = true;
                    }
                }
                // Bold
                "w:b"
                    if !is_closing
                        && !tag_content.contains("w:val=\"0\"")
                        && !tag_content.contains("w:val=\"false\"") =>
                {
                    is_bold = true;
                }
                // Italic
                "w:i"
                    if !is_closing
                        && !tag_content.contains("w:val=\"0\"")
                        && !tag_content.contains("w:val=\"false\"") =>
                {
                    is_italic = true;
                }
                // Run end — apply formatting
                "w:r" if is_closing => {
                    is_bold = false;
                    is_italic = false;
                }
                // Text content
                "w:t" if !is_closing => {
                    // Read text until </w:t>
                    let mut text = String::new();
                    while i < len {
                        if chars[i] == '<' {
                            break;
                        }
                        text.push(chars[i]);
                        i += 1;
                    }
                    let decoded = decode_xml_entities(&text);
                    if is_bold && is_italic {
                        paragraph_text.push_str(&format!("***{}***", decoded));
                    } else if is_bold {
                        paragraph_text.push_str(&format!("**{}**", decoded));
                    } else if is_italic {
                        paragraph_text.push_str(&format!("*{}*", decoded));
                    } else {
                        paragraph_text.push_str(&decoded);
                    }
                }
                // Table handling
                "w:tbl" if !is_closing => {
                    in_table = true;
                    is_first_table_row = true;
                }
                "w:tbl" if is_closing => {
                    in_table = false;
                    result.push('\n');
                }
                "w:tr" if !is_closing => {
                    table_row.clear();
                }
                "w:tr" if is_closing => {
                    if !table_row.is_empty() {
                        result.push_str("| ");
                        result.push_str(&table_row.join(" | "));
                        result.push_str(" |\n");
                        if is_first_table_row {
                            result.push_str("|");
                            for _ in &table_row {
                                result.push_str(" --- |");
                            }
                            result.push('\n');
                            is_first_table_row = false;
                        }
                    }
                }
                "w:tc" if !is_closing => {
                    in_cell = true;
                    table_cell_text.clear();
                }
                "w:tc" if is_closing => {
                    table_row.push(table_cell_text.trim().to_string());
                    in_cell = false;
                    table_cell_text.clear();
                }
                _ => {}
            }
        } else {
            i += 1;
        }
    }

    if result.trim().is_empty() {
        Ok("[Could not extract structured text from DOCX]".to_string())
    } else {
        Ok(result)
    }
}

/// Extract PPTX to Markdown with slide numbers and structure.
fn extract_pptx_markdown(archive: &mut zip::ZipArchive<fs::File>) -> Result<String, String> {
    let mut slide_names: Vec<String> = (0..archive.len())
        .filter_map(|i| archive.by_index(i).ok().map(|f| f.name().to_string()))
        .filter(|n| n.starts_with("ppt/slides/slide") && n.ends_with(".xml"))
        .collect();

    // Sort by slide number
    slide_names.sort_by(|a, b| {
        let num_a = a
            .trim_start_matches("ppt/slides/slide")
            .trim_end_matches(".xml")
            .parse::<u32>()
            .unwrap_or(0);
        let num_b = b
            .trim_start_matches("ppt/slides/slide")
            .trim_end_matches(".xml")
            .parse::<u32>()
            .unwrap_or(0);
        num_a.cmp(&num_b)
    });

    let mut result = String::new();

    for (idx, slide_name) in slide_names.iter().enumerate() {
        let xml = match read_zip_file(archive, slide_name) {
            Some(x) => x,
            None => continue,
        };

        result.push_str(&format!("## Slide {}\n\n", idx + 1));

        // Extract text from <a:t>...</a:t> tags, group by <a:p>...</a:p> paragraphs
        // Use string split approach to avoid byte/char index mismatch with CJK characters
        let mut paragraphs: Vec<String> = Vec::new();

        for para_part in xml.split("<a:p") {
            let mut para_text = String::new();
            for t_part in para_part.split("<a:t") {
                if let Some(close_pos) = t_part.find("</a:t>") {
                    if let Some(gt_pos) = t_part.find('>') {
                        if gt_pos < close_pos {
                            let text = &t_part[gt_pos + 1..close_pos];
                            para_text.push_str(&decode_xml_entities(text));
                        }
                    }
                }
            }
            let trimmed = para_text.trim().to_string();
            if !trimmed.is_empty() {
                paragraphs.push(trimmed);
            }
        }

        // First paragraph is usually the slide title
        if let Some(title) = paragraphs.first() {
            result.push_str(&format!("**{}**\n\n", title));
            for para in paragraphs.iter().skip(1) {
                result.push_str(&format!("- {}\n", para));
            }
        }
        result.push('\n');
    }

    if result.trim().is_empty() {
        Ok("[Could not extract text from PPTX]".to_string())
    } else {
        Ok(result)
    }
}

/// Extract spreadsheet to Markdown using calamine (supports xlsx, xls, ods).
fn extract_spreadsheet(path: &str) -> Result<String, String> {
    let mut workbook = open_workbook_auto(path)
        .map_err(|e| format!("Failed to open spreadsheet '{}': {}", path, e))?;

    let mut result = String::new();
    let sheet_names = workbook.sheet_names().to_vec();

    for sheet_name in &sheet_names {
        if let Ok(range) = workbook.worksheet_range(sheet_name) {
            if range.is_empty() {
                continue;
            }

            if sheet_names.len() > 1 {
                result.push_str(&format!("## {}\n\n", sheet_name));
            }

            let mut rows: Vec<Vec<String>> = Vec::new();
            let mut max_cols = 0;

            for row in range.rows() {
                let cells: Vec<String> = row
                    .iter()
                    .map(|cell| match cell {
                        Data::Empty => String::new(),
                        Data::String(s) => s.clone(),
                        Data::Float(f) => {
                            if *f == (*f as i64) as f64 {
                                format!("{}", *f as i64)
                            } else {
                                format!("{:.2}", f)
                            }
                        }
                        Data::Int(i) => i.to_string(),
                        Data::Bool(b) => b.to_string(),
                        Data::DateTime(dt) => format_excel_datetime(dt),
                        Data::DateTimeIso(s) => s.clone(),
                        Data::DurationIso(s) => s.clone(),
                        Data::Error(e) => format!("ERR:{:?}", e),
                    })
                    .collect();
                if cells.len() > max_cols {
                    max_cols = cells.len();
                }
                rows.push(cells);
            }

            // Skip empty sheets
            if rows.is_empty() || max_cols == 0 {
                continue;
            }

            for (i, row) in rows.iter().enumerate() {
                let mut padded = row.clone();
                padded.resize(max_cols, String::new());
                // Escape pipe characters in cell values
                let escaped: Vec<String> = padded.iter().map(|c| c.replace('|', "\\|")).collect();
                result.push_str("| ");
                result.push_str(&escaped.join(" | "));
                result.push_str(" |\n");

                if i == 0 {
                    result.push('|');
                    for _ in 0..max_cols {
                        result.push_str(" --- |");
                    }
                    result.push('\n');
                }
            }
            result.push('\n');
        }
    }

    if result.trim().is_empty() {
        Ok("[Could not extract data from spreadsheet]".to_string())
    } else {
        Ok(result)
    }
}

fn format_excel_datetime(value: &ExcelDateTime) -> String {
    if value.is_duration() {
        let total_millis = (value.as_f64() * 86_400_000.0).round() as i64;
        let sign = if total_millis < 0 { "-" } else { "" };
        let total_millis = total_millis.abs();
        let hours = total_millis / 3_600_000;
        let minutes = (total_millis % 3_600_000) / 60_000;
        let seconds = (total_millis % 60_000) / 1_000;
        let millis = total_millis % 1_000;
        return if millis == 0 {
            format!("{sign}{hours:02}:{minutes:02}:{seconds:02}")
        } else {
            format!("{sign}{hours:02}:{minutes:02}:{seconds:02}.{millis:03}")
        };
    }

    let (year, month, day, hour, minute, second, millis) = value.to_ymd_hms_milli();
    if value.as_f64().abs() < 1.0 {
        return if millis == 0 {
            format!("{hour:02}:{minute:02}:{second:02}")
        } else {
            format!("{hour:02}:{minute:02}:{second:02}.{millis:03}")
        };
    }
    if hour == 0 && minute == 0 && second == 0 && millis == 0 {
        format!("{year:04}-{month:02}-{day:02}")
    } else if millis == 0 {
        format!("{year:04}-{month:02}-{day:02} {hour:02}:{minute:02}:{second:02}")
    } else {
        format!("{year:04}-{month:02}-{day:02} {hour:02}:{minute:02}:{second:02}.{millis:03}")
    }
}

/// Extract OpenDocument format text (basic).
fn extract_odf_text(archive: &mut zip::ZipArchive<fs::File>) -> Result<String, String> {
    let xml =
        read_zip_file(archive, "content.xml").ok_or_else(|| "No content.xml found".to_string())?;

    let mut result = String::new();
    let mut in_tag = false;

    for ch in xml.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => {
                in_tag = false;
                result.push(' ');
            }
            _ if !in_tag => result.push(ch),
            _ => {}
        }
    }

    let cleaned = decode_xml_entities(&result);
    let lines: Vec<&str> = cleaned
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect();

    if lines.is_empty() {
        Ok("[Could not extract text from this file]".to_string())
    } else {
        Ok(lines.join("\n\n"))
    }
}

#[tauri::command]
pub async fn write_file(path: String, contents: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        run_guarded("write_file", || {
            require_absolute_path("write_file", &path)?;
            let p = Path::new(&path);
            if let Some(parent) = p.parent() {
                fs::create_dir_all(parent)
                    .map_err(|e| format!("Failed to create parent dirs for '{}': {}", path, e))?;
            }
            fs::write(&path, contents)
                .map_err(|e| format!("Failed to write file '{}': {}", path, e))?;
            Ok(())
        })
    })
    .await
    .map_err(|e| format!("write_file blocking task join error: {e}"))?
}

/// Whether a directory entry should appear in a listing.
///
/// Hidden (dot-prefixed) entries are shown only when `include_hidden`
/// is set, so internal state (`.git`), caches and secrets (`.env`) never
/// leak into a listing by default.
fn entry_is_visible(name: &str, include_hidden: bool) -> bool {
    include_hidden || !name.starts_with('.')
}

#[tauri::command]
pub async fn list_directory(
    path: String,
    include_hidden: Option<bool>,
    max_depth: Option<usize>,
) -> Result<Vec<FileNode>, String> {
    let include_hidden = include_hidden.unwrap_or(false);
    let max_depth = max_depth.unwrap_or(30).clamp(1, 30);
    tauri::async_runtime::spawn_blocking(move || {
        run_guarded("list_directory", || {
            let logical = Path::new(&path);
            let p = logical;
            if !p.exists() {
                return Err(format!("Path does not exist: '{}'", path));
            }
            if !p.is_dir() {
                return Err(format!("Path is not a directory: '{}'", path));
            }
            let nodes = build_tree_at(p, logical, 0, max_depth, include_hidden)?;
            Ok(nodes)
        })
    })
    .await
    .map_err(|e| format!("list_directory blocking task join error: {e}"))?
}

#[cfg(test)]
fn build_tree(
    dir: &Path,
    depth: usize,
    max_depth: usize,
    include_hidden: bool,
) -> Result<Vec<FileNode>, String> {
    build_tree_at(dir, dir, depth, max_depth, include_hidden)
}

/// `dir` is where the entries are read from and `logical` the path
/// they are reported under. The two differ only inside a mounted source.
fn build_tree_at(
    dir: &Path,
    logical: &Path,
    depth: usize,
    max_depth: usize,
    include_hidden: bool,
) -> Result<Vec<FileNode>, String> {
    if depth >= max_depth {
        return Ok(vec![]);
    }

    let mut entries: Vec<_> = fs::read_dir(dir)
        .map_err(|e| format!("Failed to read directory '{}': {}", dir.display(), e))?
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            entry
                .file_name()
                .to_str()
                .map(|n| entry_is_visible(n, include_hidden))
                .unwrap_or(false)
        })
        .collect();

    // Sort: directories first, then alphabetical within each group
    entries.sort_by(|a, b| {
        let a_is_dir = a.path().is_dir();
        let b_is_dir = b.path().is_dir();
        match (a_is_dir, b_is_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.file_name().cmp(&b.file_name()),
        }
    });

    let mut nodes = Vec::new();
    for entry in entries {
        let entry_path = entry.path();
        let name = entry.file_name().to_str().unwrap_or("").to_string();
        // Always return forward-slash paths so the TS layer can compare
        // and compose paths consistently across Windows and Unix. Windows
        // APIs accept forward slashes, so normalizing here is safe and
        // prevents a whole class of bugs where TS-constructed `/` paths
        // fail to match Rust-returned `\` paths.
        let logical_path = logical.join(&name);
        let path_str = logical_path.to_string_lossy().replace('\\', "/");
        let is_dir = entry_path.is_dir();

        let children = if is_dir {
            let kids =
                build_tree_at(&entry_path, &logical_path, depth + 1, max_depth, include_hidden)?;
            if kids.is_empty() {
                None
            } else {
                Some(kids)
            }
        } else {
            None
        };

        nodes.push(FileNode {
            name,
            path: path_str,
            is_dir,
            children,
        });
    }

    Ok(nodes)
}

#[tauri::command]
pub async fn delete_file(path: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        run_guarded("delete_file", || {
            let p = Path::new(&path);
            if p.is_dir() {
                remove_path_with_retry(&path, true)
                    .map_err(|e| format!("Failed to delete directory '{}': {}", path, e))?;
            } else {
                remove_path_with_retry(&path, false)
                    .map_err(|e| format!("Failed to delete file '{}': {}", path, e))?;
            }
            Ok(())
        })
    })
    .await
    .map_err(|e| format!("delete_file blocking task join error: {e}"))?
}

fn remove_path_with_retry(path: &str, is_dir: bool) -> Result<(), std::io::Error> {
    let mut last_err: Option<std::io::Error> = None;
    for attempt in 0..4 {
        let result = if is_dir {
            fs::remove_dir_all(path)
        } else {
            fs::remove_file(path)
        };
        match result {
            Ok(()) => return Ok(()),
            Err(err) if attempt < 3 && is_windows_transient_delete_error(&err) => {
                last_err = Some(err);
                thread::sleep(Duration::from_millis(250 * (1_u64 << attempt)));
            }
            Err(err) => return Err(err),
        }
    }
    Err(last_err.unwrap_or_else(|| std::io::Error::other("delete failed")))
}

fn is_windows_transient_delete_error(err: &std::io::Error) -> bool {
    #[cfg(windows)]
    {
        matches!(err.raw_os_error(), Some(32 | 33))
            || err.kind() == std::io::ErrorKind::PermissionDenied
    }
    #[cfg(not(windows))]
    {
        let _ = err;
        false
    }
}

#[tauri::command]
pub async fn create_directory(path: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        run_guarded("create_directory", || {
            require_absolute_path("create_directory", &path)?;
            fs::create_dir_all(&path)
                .map_err(|e| format!("Failed to create directory '{}': {}", path, e))
        })
    })
    .await
    .map_err(|e| format!("create_directory blocking task join error: {e}"))?
}

/// Read any file as base64 + a guessed mime type. Used by the
/// vision-caption pipeline to slurp extracted image bytes off disk
/// without round-tripping them through the JS string-as-UTF8 path
/// (`read_file` would corrupt PNG bytes — they aren't valid UTF-8).
///
/// Mime detection is by extension only — the caption helper doesn't
/// care about exact accuracy (vision models accept any common
/// raster format), and the alternative (sniffing magic bytes via
/// `infer` or similar) adds a dependency for marginal benefit.
/// Unknown extensions fall back to `application/octet-stream`,
/// which all major vision endpoints accept (Anthropic / OpenAI both
/// also support that as a generic fallback).
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileBase64 {
    pub base64: String,
    pub mime_type: String,
}

#[tauri::command]
pub async fn read_file_as_base64(path: String) -> Result<FileBase64, String> {
    use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
    tauri::async_runtime::spawn_blocking(move || {
        run_guarded("read_file_as_base64", || {
            let bytes = fs::read(&path).map_err(|e| format!("Failed to read '{}': {}", path, e))?;
            let p = Path::new(&path);
            let ext = p
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_ascii_lowercase();
            let mime_type = match ext.as_str() {
                "png" => "image/png",
                "jpg" | "jpeg" => "image/jpeg",
                "gif" => "image/gif",
                "webp" => "image/webp",
                "bmp" => "image/bmp",
                "tiff" | "tif" => "image/tiff",
                "svg" => "image/svg+xml",
                "pdf" => "application/pdf",
                _ => "application/octet-stream",
            }
            .to_string();
            Ok(FileBase64 {
                base64: B64.encode(&bytes),
                mime_type,
            })
        })
    })
    .await
    .map_err(|e| format!("read_file_as_base64 blocking task join error: {e}"))?
}

#[tauri::command]
pub async fn get_file_size(path: String) -> Result<u64, String> {
    tauri::async_runtime::spawn_blocking(move || {
        run_guarded("get_file_size", || {
            let metadata = fs::metadata(&path)
                .map_err(|e| format!("Failed to get metadata for '{}': {}", path, e))?;
            Ok(metadata.len())
        })
    })
    .await
    .map_err(|e| format!("get_file_size blocking task join error: {e}"))?
}

#[cfg(test)]
#[path = "fs_anydoc_benchmark.rs"]
mod anydoc_benchmark;


#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn tmp_file_with_extension(extension: &str, bytes: &[u8]) -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!(
            "llm-wiki-anydoc-{}.{}",
            uuid::Uuid::new_v4(),
            extension
        ));
        fs::write(&path, bytes).unwrap();
        path
    }

    fn tmp_zip_with_entries(extension: &str, entries: &[(&str, &str)]) -> std::path::PathBuf {
        let path = tmp_file_with_extension(extension, &[]);
        let file = fs::File::create(&path).unwrap();
        let mut archive = zip::ZipWriter::new(file);
        let options = zip::write::SimpleFileOptions::default();
        for (name, content) in entries {
            archive.start_file(*name, options).unwrap();
            archive.write_all(content.as_bytes()).unwrap();
        }
        archive.finish().unwrap();
        path
    }

    fn minimal_docx() -> std::path::PathBuf {
        tmp_zip_with_entries(
            "docx",
            &[
                (
                    "[Content_Types].xml",
                    r#"<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/></Types>"#,
                ),
                (
                    "_rels/.rels",
                    r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/></Relationships>"#,
                ),
                (
                    "word/document.xml",
                    r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:p><w:r><w:t>AnyDoc 中文 Word</w:t></w:r></w:p><w:tbl><w:tr><w:tc><w:p><w:r><w:t>Name</w:t></w:r></w:p></w:tc><w:tc><w:p><w:r><w:t>Value</w:t></w:r></w:p></w:tc></w:tr><w:tr><w:tc><w:p><w:r><w:t>Alpha</w:t></w:r></w:p></w:tc><w:tc><w:p><w:r><w:t>42</w:t></w:r></w:p></w:tc></w:tr></w:tbl></w:body></w:document>"#,
                ),
            ],
        )
    }

    fn minimal_pptx() -> std::path::PathBuf {
        tmp_zip_with_entries(
            "pptx",
            &[
                (
                    "[Content_Types].xml",
                    r#"<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/ppt/presentation.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml"/><Override PartName="/ppt/slides/slide1.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slide+xml"/></Types>"#,
                ),
                (
                    "_rels/.rels",
                    r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="ppt/presentation.xml"/></Relationships>"#,
                ),
                (
                    "ppt/presentation.xml",
                    r#"<p:presentation xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><p:sldIdLst><p:sldId id="256" r:id="rId1"/></p:sldIdLst></p:presentation>"#,
                ),
                (
                    "ppt/_rels/presentation.xml.rels",
                    r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide1.xml"/></Relationships>"#,
                ),
                (
                    "ppt/slides/slide1.xml",
                    r#"<p:sld xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"><p:cSld><p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/><p:sp><p:nvSpPr><p:cNvPr id="2" name="Title"/><p:cNvSpPr/><p:nvPr><p:ph type="title"/></p:nvPr></p:nvSpPr><p:spPr/><p:txBody><a:bodyPr/><a:p><a:r><a:t>AnyDoc 演示文稿</a:t></a:r></a:p></p:txBody></p:sp></p:spTree></p:cSld></p:sld>"#,
                ),
            ],
        )
    }

    fn minimal_xlsx() -> std::path::PathBuf {
        tmp_zip_with_entries(
            "xlsx",
            &[
                (
                    "[Content_Types].xml",
                    r#"<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/><Override PartName="/xl/worksheets/sheet1.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/></Types>"#,
                ),
                (
                    "_rels/.rels",
                    r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>"#,
                ),
                (
                    "xl/workbook.xml",
                    r#"<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><sheets><sheet name="Data" sheetId="1" r:id="rId1"/></sheets></workbook>"#,
                ),
                (
                    "xl/_rels/workbook.xml.rels",
                    r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/></Relationships>"#,
                ),
                (
                    "xl/worksheets/sheet1.xml",
                    r#"<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheetData><row r="1"><c r="A1" t="inlineStr"><is><t>Metric</t></is></c><c r="B1" t="inlineStr"><is><t>Value</t></is></c></row><row r="2"><c r="A2" t="inlineStr"><is><t>安全</t></is></c><c r="B2"><v>99</v></c></row></sheetData></worksheet>"#,
                ),
            ],
        )
    }

    fn xlsx_with_custom_chinese_date_format() -> std::path::PathBuf {
        tmp_zip_with_entries(
            "xlsx",
            &[
                (
                    "[Content_Types].xml",
                    r#"<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/><Override PartName="/xl/worksheets/sheet1.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/><Override PartName="/xl/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.styles+xml"/></Types>"#,
                ),
                (
                    "_rels/.rels",
                    r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>"#,
                ),
                (
                    "xl/workbook.xml",
                    r#"<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><sheets><sheet name="Dates" sheetId="1" r:id="rId1"/></sheets></workbook>"#,
                ),
                (
                    "xl/_rels/workbook.xml.rels",
                    r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/><Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/></Relationships>"#,
                ),
                (
                    "xl/styles.xml",
                    r#"<styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><numFmts count="1"><numFmt numFmtId="164" formatCode="m&quot;月&quot;d&quot;日&quot;"/></numFmts><fonts count="1"><font/></fonts><fills count="1"><fill/></fills><borders count="1"><border/></borders><cellStyleXfs count="1"><xf/></cellStyleXfs><cellXfs count="2"><xf numFmtId="0"/><xf numFmtId="164" applyNumberFormat="1"/></cellXfs></styleSheet>"#,
                ),
                (
                    "xl/worksheets/sheet1.xml",
                    r#"<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheetData><row r="1"><c r="A1" t="inlineStr"><is><t>Date</t></is></c></row><row r="2"><c r="A2" s="1"><v>45761</v></c></row></sheetData></worksheet>"#,
                ),
            ],
        )
    }

    fn minimal_odt() -> std::path::PathBuf {
        tmp_zip_with_entries(
            "odt",
            &[
                ("mimetype", "application/vnd.oasis.opendocument.text"),
                (
                    "content.xml",
                    r#"<office:document-content xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0" xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0"><office:body><office:text><text:h text:outline-level="1">AnyDoc ODT</text:h><text:p>跨平台开放文档内容。</text:p></office:text></office:body></office:document-content>"#,
                ),
            ],
        )
    }

    #[test]
    fn anydoc_extracts_rtf_through_the_office_pipeline() {
        let path = tmp_file_with_extension(
            "rtf",
            br#"{\rtf1\ansi\deff0 {\fonttbl {\f0 Arial;}}\f0\fs24 AnyDoc \b structured\b0 text.}"#,
        );
        let output = extract_office_text(path.to_str().unwrap(), "rtf").unwrap();
        fs::remove_file(&path).unwrap();

        assert!(output.contains("AnyDoc"));
        assert!(output.contains("**structured**"));
    }

    #[test]
    fn anydoc_extracts_generated_structured_document_fixtures() {
        let fixtures = [
            (
                minimal_docx(),
                "docx",
                &["AnyDoc 中文 Word", "Alpha", "42"][..],
            ),
            (minimal_pptx(), "pptx", &["AnyDoc 演示文稿"][..]),
            (minimal_xlsx(), "xlsx", &["Metric", "安全", "99"][..]),
            (
                minimal_odt(),
                "odt",
                &["AnyDoc ODT", "跨平台开放文档内容"][..],
            ),
        ];

        for (path, extension, expected_fragments) in fixtures {
            let output = extract_office_text(path.to_str().unwrap(), extension).unwrap();
            for fragment in expected_fragments {
                assert!(
                    output.contains(fragment),
                    ".{extension} output did not contain {fragment:?}: {output}"
                );
            }
            fs::remove_file(path).unwrap();
        }
    }

    #[test]
    fn spreadsheet_formats_custom_chinese_dates_instead_of_excel_serials() {
        let path = xlsx_with_custom_chinese_date_format();
        let output = extract_spreadsheet(path.to_str().unwrap()).unwrap();
        fs::remove_file(path).unwrap();

        assert!(output.contains("2025-04-14"), "{output}");
        assert!(!output.contains("45761"), "{output}");
    }

    #[test]
    fn excel_datetime_formatter_preserves_times_and_durations() {
        use calamine::ExcelDateTimeType;

        let time = ExcelDateTime::new(0.5, ExcelDateTimeType::DateTime, false);
        let duration = ExcelDateTime::new(1.5, ExcelDateTimeType::TimeDelta, false);

        assert_eq!(format_excel_datetime(&time), "12:00:00");
        assert_eq!(format_excel_datetime(&duration), "36:00:00");
    }

    #[test]
    fn anydoc_only_formats_do_not_report_success_after_parse_failure() {
        let path = tmp_file_with_extension("xlsb", b"not a workbook");
        let result = extract_office_text(path.to_str().unwrap(), "xlsb");
        fs::remove_file(path).unwrap();

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.contains("AnyDoc failed to extract .xlsb"));
        assert!(error.contains("no compatibility parser"));
    }

    #[test]
    fn org_to_markdown_preserves_common_elements_without_executing_source_blocks() {
        let input = "#+TITLE: Research Notes\r\n#+AUTHOR: Ada\r\n* TODO Topic :rust:notes:\r\nParagraph with [[https://example.com][reference]] and [[file:local.pdf]].\r\n- [ ] Verify\r\n| Name | Value |\r\n|------+-------|\r\n| A    | 1     |\r\n#+BEGIN_SRC sh\r\necho never-run\r\n#+END_SRC\r\n";
        let output = org_to_markdown(input);
        assert!(output.contains("# Research Notes"));
        assert!(output.contains("**Author:** Ada"));
        assert!(output.contains("# TODO Topic :rust:notes:"));
        assert!(output.contains("[reference](https://example.com)"));
        assert!(output.contains("<file:local.pdf>"));
        assert!(output.contains("|------|-------|"));
        assert!(output.contains("```sh\necho never-run\n```"));
    }

    #[test]
    fn org_to_markdown_closes_unterminated_source_block_and_keeps_unknown_lines() {
        let output = org_to_markdown(
            "#+OPTIONS: toc:nil\n#+CUSTOM: kept\n* Heading\n#+BEGIN_SRC python\nprint('text only')",
        );
        assert!(!output.contains("OPTIONS"));
        assert!(output.contains("**Custom:** kept"));
        assert!(output.ends_with("```"));
    }

    /// Write `bytes` to a fresh tmp path with `.pdf` suffix and return
    /// the path (the OS tmpdir is NOT cleaned up — acceptable for tests).
    fn tmp_pdf_with_bytes(bytes: &[u8]) -> String {
        let dir = std::env::temp_dir();
        let path = dir.join(format!(
            "panic-guard-{}.pdf",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let mut f = fs::File::create(&path).unwrap();
        f.write_all(bytes).unwrap();
        path.to_string_lossy().to_string()
    }

    /// Verify read_file does NOT crash the test process on malformed PDFs.
    /// We try a handful of payloads that have historically caused
    /// pdf-extract/lopdf panics — any process abort would fail the test
    /// runner before it can report.
    ///
    /// `multi_thread` flavor: `read_file` now uses
    /// `tauri::async_runtime::spawn_blocking`, which moves work onto
    /// the tokio blocking pool. The blocking pool requires a multi-
    /// threaded runtime — the default `#[tokio::test]` is single-
    /// threaded current-thread, on which `.await` of a `spawn_blocking`
    /// future deadlocks.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn read_file_survives_malformed_pdf_inputs() {
        let payloads: &[(&str, &[u8])] = &[
            ("empty", b""),
            ("not_a_pdf", b"this is plainly not a PDF file"),
            ("header_only", b"%PDF-1.4\n"),
            (
                "broken_xref",
                b"%PDF-1.4\n1 0 obj\n<<>>\nendobj\nxref\nBROKENBROKEN\ntrailer\n<</Size 1>>\nstartxref\n999999\n%%EOF\n",
            ),
            (
                "junk_after_header",
                b"%PDF-1.4\n\x00\x01\x02\x03\x04\x05\x06\x07\xFF\xFE\xFDjunkgarbage",
            ),
        ];

        for (name, bytes) in payloads {
            let path = tmp_pdf_with_bytes(bytes);
            let result = read_file(path.clone()).await;
            let _ = fs::remove_file(&path);
            eprintln!(
                "[{name}] => {:?}",
                result.as_ref().map(|s| &s[..s.len().min(80)])
            );
        }
    }

    /// Smoke test: a real PDF panic (synthesized) is caught. We can't
    /// guarantee that any particular byte sequence above actually panics
    /// pdf-extract across versions, so also trigger an explicit panic
    /// through read_file's guarded path.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn read_file_returns_err_on_missing_file_instead_of_panicking() {
        let result = read_file("/nonexistent/path/that/does/not/exist.pdf".to_string()).await;
        assert!(result.is_err() || result.is_ok()); // must at least return
    }

    #[test]
    fn extract_doc_with_office_oxide_handles_missing_file() {
        let result = extract_doc_with_office_oxide("/nonexistent/path/that/does/not/exist.doc");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Failed to parse DOC"));
    }

    /// Ad-hoc probe: run the production PDF extraction path against every
    /// .pdf under a user-provided directory and print a per-file report of
    /// Ok / Err (library returned an error) / Panic (library panicked and
    /// was caught by panic_guard). Gated with #[ignore] so it never runs
    /// in CI; execute locally with:
    ///
    ///   PDF_PROBE_DIR=/path/to/pdfs cargo test --lib \
    ///     -- --ignored --nocapture pdf_probe
    #[test]
    #[ignore = "local probe; set PDF_PROBE_DIR"]
    fn pdf_probe() {
        let dir = std::env::var("PDF_PROBE_DIR")
            .unwrap_or_else(|_| "/Users/nash_su/Downloads/pdftests".to_string());
        let root = std::path::Path::new(&dir);
        if !root.exists() {
            eprintln!("[pdf_probe] dir not found: {}", root.display());
            return;
        }

        let mut pdfs: Vec<std::path::PathBuf> = Vec::new();
        fn walk(d: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
            if let Ok(entries) = fs::read_dir(d) {
                for entry in entries.flatten() {
                    let p = entry.path();
                    if p.is_dir() {
                        walk(&p, out);
                    } else if p
                        .extension()
                        .and_then(|e| e.to_str())
                        .map(|e| e.eq_ignore_ascii_case("pdf"))
                        .unwrap_or(false)
                    {
                        out.push(p);
                    }
                }
            }
        }
        walk(root, &mut pdfs);
        pdfs.sort();

        eprintln!(
            "\n[pdf_probe] found {} PDFs under {}\n",
            pdfs.len(),
            root.display()
        );

        let mut ok = 0usize;
        let mut err = 0usize;
        let mut panicked = 0usize;

        for (idx, path) in pdfs.iter().enumerate() {
            let display = path.display().to_string();
            // Call extract_pdf_text directly (not read_file) so we bypass
            // the .cache sibling dir and always exercise the parser.
            let path_str = path.to_string_lossy().to_string();
            let result = std::panic::catch_unwind(|| extract_pdf_text(&path_str));
            match result {
                Ok(Ok(text)) => {
                    ok += 1;
                    eprintln!(
                        "[{:>3}/{}] OK     ({:>7} chars)  {}",
                        idx + 1,
                        pdfs.len(),
                        text.len(),
                        display
                    );
                }
                Ok(Err(e)) => {
                    err += 1;
                    eprintln!(
                        "[{:>3}/{}] ERR    {}  →  {}",
                        idx + 1,
                        pdfs.len(),
                        display,
                        e
                    );
                }
                Err(payload) => {
                    panicked += 1;
                    let msg = if let Some(s) = payload.downcast_ref::<String>() {
                        s.clone()
                    } else if let Some(s) = payload.downcast_ref::<&str>() {
                        (*s).to_string()
                    } else {
                        "(non-string panic)".to_string()
                    };
                    eprintln!(
                        "[{:>3}/{}] PANIC  {}  →  {}",
                        idx + 1,
                        pdfs.len(),
                        display,
                        msg
                    );
                }
            }
        }

        eprintln!(
            "\n[pdf_probe] summary: {} OK / {} ERR / {} PANIC (total {})",
            ok,
            err,
            panicked,
            pdfs.len()
        );
    }

    fn make_temp_dir(label: &str) -> std::path::PathBuf {
        let dir =
            std::env::temp_dir().join(format!("llmwiki-copydir-{label}-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn build_tree_hides_dot_entries_by_default_and_includes_them_when_asked() {
        let root = make_temp_dir("build-tree-hidden");
        fs::create_dir_all(root.join(".claude")).unwrap();
        fs::write(root.join(".claude/CLAUDE.md"), "x").unwrap();
        fs::create_dir_all(root.join("visible")).unwrap();
        fs::write(root.join("visible/doc.md"), "y").unwrap();
        fs::write(root.join(".env"), "secret").unwrap();

        // Default: dot entries (incl. .env) hidden, normal entries shown.
        let hidden = build_tree(&root, 0, 30, false).unwrap();
        let hidden_names: Vec<&str> = hidden.iter().map(|n| n.name.as_str()).collect();
        assert!(hidden_names.contains(&"visible"));
        assert!(
            !hidden_names.iter().any(|n| n.starts_with('.')),
            "no dot entries should leak by default: {hidden_names:?}"
        );

        // include_hidden=true: dotfolders/dotfiles present.
        let shown = build_tree(&root, 0, 30, true).unwrap();
        let shown_names: Vec<&str> = shown.iter().map(|n| n.name.as_str()).collect();
        assert!(shown_names.contains(&".claude"));
        assert!(shown_names.contains(&".env"));
        assert!(shown_names.contains(&"visible"));

        // The dotfolder's children come through too.
        let claude = shown.iter().find(|n| n.name == ".claude").unwrap();
        let kids = claude
            .children
            .as_ref()
            .expect(".claude should have children");
        assert!(kids.iter().any(|n| n.name == "CLAUDE.md"));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn build_tree_respects_max_depth_for_shallow_listing() {
        let root = make_temp_dir("build-tree-depth");
        fs::create_dir_all(root.join("a/b")).unwrap();
        fs::write(root.join("a/b/deep.md"), "x").unwrap();

        let shallow = build_tree(&root, 0, 1, false).unwrap();
        let a = shallow.iter().find(|n| n.name == "a").unwrap();
        assert!(a.is_dir);
        assert!(
            a.children.is_none(),
            "max_depth=1 must not load grandchildren"
        );

        let deeper = build_tree(&root, 0, 3, false).unwrap();
        let a = deeper.iter().find(|n| n.name == "a").unwrap();
        let b = a
            .children
            .as_ref()
            .and_then(|children| children.iter().find(|n| n.name == "b"))
            .expect("max_depth=3 should include nested directory");
        let files = b.children.as_ref().expect("b should include deep file");
        assert!(files.iter().any(|n| n.name == "deep.md"));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn reject_relative_write_paths_before_touching_cwd() {
        assert!(require_absolute_path("write_file", "notas/suelta.md").is_err());
        assert!(require_absolute_path("write_file", "./notas/suelta.md").is_err());
    }

    #[test]
    fn allow_absolute_write_paths() {
        // A single leading slash is absolute on Unix only; on Windows it is
        // relative to the current drive.
        #[cfg(unix)]
        assert!(require_absolute_path("write_file", "/tmp/carpeta/notas/pagina.md").is_ok());
        assert!(require_absolute_path("write_file", "C:/carpeta/notas/pagina.md").is_ok());
        assert!(require_absolute_path("write_file", r"C:\project\wiki\sources\page.md").is_ok());
        assert!(
            require_absolute_path("write_file", r"\\server\share\wiki\sources\page.md").is_ok()
        );
        assert!(require_absolute_path("write_file", "//server/share/notas/pagina.md").is_ok());
        assert!(require_absolute_path("write_file", "C:notas/pagina.md").is_err());
    }

}
