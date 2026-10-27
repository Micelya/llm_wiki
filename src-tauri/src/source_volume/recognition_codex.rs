//! Text recognition, and questions about a document's text, through the
//! locally installed Codex CLI, using whatever login that CLI already has.
//!
//! One `codex exec` run per image or per question. The agent is given
//! nothing but the image or the question: it runs in an empty scratch
//! directory, in a read-only sandbox and with approvals disabled, so text
//! inside a document cannot talk it into touching the user's files.

use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use super::asking::DocumentAsker;
use super::recognition::TextRecognizer;

const TIMEOUT: Duration = Duration::from_secs(300);
const STDERR_TAIL_BYTES: usize = 2000;

/// What is asked about each image; Ajustes shows it as it is
/// (`docbase/questions.rs`).
pub(crate) const PROMPT: &str = "\
Transcribí todo el texto visible en la imagen adjunta. Es una página de un documento.

Reglas:
- Transcripción literal y completa, en el idioma original. No resumas, no corrijas, no traduzcas y no agregues comentarios.
- Respetá el orden de lectura y los saltos de línea. Usá Markdown solo para reflejar la estructura que ya existe: títulos y tablas.
- Copiá números, importes, fechas, códigos y nombres carácter por carácter. Si un carácter no se distingue, no lo adivines: escribí [ilegible] en su lugar.
- Indicá entre corchetes los elementos que no son texto y tienen valor documental: [firma], [sello: texto del sello], [logo: nombre], [manuscrito: texto].
- Si la imagen no contiene texto, respondé exactamente: [sin texto]
- No ejecutes comandos ni leas otros archivos.

Respondé únicamente con la transcripción.";

pub struct CodexRecognizer {
    model: String,
}

impl CodexRecognizer {
    pub fn new(model: &str) -> Self {
        Self {
            model: model.trim().to_string(),
        }
    }

    fn args(&self, image: &Path, answer_file: &Path) -> Vec<std::ffi::OsString> {
        // `-i` takes a list, so it goes first and the next flag ends it.
        let mut args: Vec<std::ffi::OsString> = vec![
            "-a".into(),
            "never".into(),
            "exec".into(),
            "-i".into(),
            image.into(),
            "--skip-git-repo-check".into(),
            "--sandbox".into(),
            "read-only".into(),
            "--ephemeral".into(),
        ];
        if !self.model.is_empty() {
            args.push("--model".into());
            args.push((&self.model).into());
        }
        args.push("-o".into());
        args.push(answer_file.into());
        // Prompt is read from stdin.
        args.push("-".into());
        args
    }
}

impl TextRecognizer for CodexRecognizer {
    fn engine_id(&self) -> String {
        if self.model.is_empty() {
            "codex-cli".to_string()
        } else {
            format!("codex-cli:{}", self.model)
        }
    }

    fn recognize(&self, image: &Path) -> Result<String, String> {
        let codex = find_codex()?;
        let scratch =
            std::env::temp_dir().join(format!("micelya-codex-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&scratch)
            .map_err(|e| format!("Failed to create '{}': {e}", scratch.display()))?;
        let answer_file = scratch.join("respuesta.txt");

        let result = run_codex(&codex, &self.args(image, &answer_file), &scratch, PROMPT).and_then(|()| {
            let text = fs::read_to_string(&answer_file)
                .map_err(|e| format!("Codex did not write an answer: {e}"))?;
            if text.trim().is_empty() {
                return Err("Codex returned an empty answer".to_string());
            }
            Ok(text)
        });
        let _ = fs::remove_dir_all(&scratch);
        result
    }
}

/// Questions about a document's text (its type, its data), answered in
/// the JSON shape the caller gives; Codex enforces it with
/// `--output-schema`.
pub struct CodexAsker {
    model: String,
    /// `low`, `medium` or `high` (checked by `asking::asker_for`); empty
    /// leaves the model's default.
    reasoning: String,
}

impl CodexAsker {
    pub fn new(model: &str) -> Self {
        Self {
            model: model.trim().to_string(),
            reasoning: String::new(),
        }
    }

    pub fn with_reasoning(mut self, reasoning: &str) -> Self {
        self.reasoning = reasoning.trim().to_lowercase();
        self
    }

    fn args(&self, schema_file: &Path, answer_file: &Path, images: &[PathBuf]) -> Vec<std::ffi::OsString> {
        let mut args: Vec<std::ffi::OsString> = vec!["-a".into(), "never".into(), "exec".into()];
        // `-i` takes a list, so it goes first and the next flag ends it.
        if !images.is_empty() {
            args.push("-i".into());
            args.extend(images.iter().map(|i| i.as_os_str().to_os_string()));
        }
        args.extend::<Vec<std::ffi::OsString>>(vec![
            "--skip-git-repo-check".into(),
            "--sandbox".into(),
            "read-only".into(),
            "--ephemeral".into(),
            "--output-schema".into(),
            schema_file.into(),
        ]);
        if !self.model.is_empty() {
            args.push("--model".into());
            args.push((&self.model).into());
        }
        if !self.reasoning.is_empty() {
            // Codex's own setting; a bare word is taken as a string.
            args.push("-c".into());
            args.push(format!("model_reasoning_effort={}", self.reasoning).into());
        }
        args.push("-o".into());
        args.push(answer_file.into());
        args.push("-".into());
        args
    }
}

impl DocumentAsker for CodexAsker {
    fn engine_id(&self) -> String {
        let model = super::asking::model_with_reasoning(&self.model, &self.reasoning);
        if model.is_empty() {
            "codex-cli".to_string()
        } else {
            format!("codex-cli:{model}")
        }
    }

    fn ask(&self, prompt: &str, schema: &serde_json::Value) -> Result<String, String> {
        self.ask_with_images(prompt, schema, &[])
    }

    fn sees_images(&self) -> bool {
        true
    }

    fn ask_with_images(&self, prompt: &str, schema: &serde_json::Value, images: &[PathBuf]) -> Result<String, String> {
        if let Some(missing) = images.iter().find(|i| !i.is_absolute() || !i.exists()) {
            // Codex does not fail on an image it cannot find: it answers without it.
            return Err(format!("The image '{}' cannot be shown to the model", missing.display()));
        }
        let codex = find_codex()?;
        let scratch = std::env::temp_dir().join(format!("micelya-codex-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&scratch).map_err(|e| format!("Failed to create '{}': {e}", scratch.display()))?;
        let schema_file = scratch.join("forma.json");
        let answer_file = scratch.join("respuesta.json");
        let result = fs::write(&schema_file, schema.to_string())
            .map_err(|e| format!("Failed to write the answer's shape: {e}"))
            .and_then(|()| run_codex(&codex, &self.args(&schema_file, &answer_file, images), &scratch, prompt))
            .and_then(|()| {
                let text = fs::read_to_string(&answer_file).map_err(|e| format!("Codex did not write an answer: {e}"))?;
                if text.trim().is_empty() {
                    return Err("Codex returned an empty answer".to_string());
                }
                Ok(text)
            });
        let _ = fs::remove_dir_all(&scratch);
        result
    }
}

fn find_codex() -> Result<PathBuf, String> {
    let candidates: &[&str] = if cfg!(windows) {
        &["codex.cmd", "codex.exe", "codex"]
    } else {
        &["codex"]
    };
    candidates
        .iter()
        .find_map(|name| which::which(name).ok())
        .ok_or_else(|| "`codex` not found on PATH".to_string())
}

fn run_codex(codex: &Path, args: &[std::ffi::OsString], cwd: &Path, prompt: &str) -> Result<(), String> {
    let mut cmd = Command::new(codex);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd.args(args)
        .current_dir(cwd)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    let mut child = cmd
        .spawn()
        .map_err(|e| format!("Failed to start codex: {e}"))?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(prompt.as_bytes())
            .map_err(|e| format!("Failed to send the prompt to codex: {e}"))?;
    }
    // Drain stderr on its own thread so a chatty run cannot fill the pipe
    // and stall the process.
    let stderr = child.stderr.take();
    let stderr_reader = std::thread::spawn(move || {
        let mut collected = String::new();
        if let Some(mut stderr) = stderr {
            let _ = stderr.read_to_string(&mut collected);
        }
        collected
    });

    let started = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if started.elapsed() > TIMEOUT => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!(
                    "Codex did not answer within {} seconds",
                    TIMEOUT.as_secs()
                ));
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(200)),
            Err(e) => return Err(format!("Failed to wait for codex: {e}")),
        }
    };
    if status.success() {
        return Ok(());
    }
    let stderr = stderr_reader.join().unwrap_or_default();
    let tail_start = stderr.len().saturating_sub(STDERR_TAIL_BYTES);
    let tail_start = (tail_start..=stderr.len())
        .find(|i| stderr.is_char_boundary(*i))
        .unwrap_or(stderr.len());
    Err(format!(
        "Codex exited with {status}: {}",
        stderr[tail_start..].trim()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn image_flag_is_closed_by_the_next_flag_and_prompt_comes_from_stdin() {
        let recognizer = CodexRecognizer::new(" gpt-6-luna ");
        let args = recognizer
            .args(Path::new("pagina 1.png"), Path::new("out.txt"))
            .into_iter()
            .map(|a| a.to_string_lossy().to_string())
            .collect::<Vec<_>>();

        let image = args.iter().position(|a| a == "-i").unwrap();
        assert_eq!(args[image + 1], "pagina 1.png");
        assert!(args[image + 2].starts_with("--"));
        assert_eq!(args.last().unwrap(), "-");
        assert!(args.windows(2).any(|w| w == ["--sandbox", "read-only"]));
        assert!(args.windows(2).any(|w| w == ["-a", "never"]));
        assert!(args.windows(2).any(|w| w == ["--model", "gpt-6-luna"]));
        assert_eq!(recognizer.engine_id(), "codex-cli:gpt-6-luna");
    }

    #[test]
    fn a_question_carries_its_answer_shape_and_comes_from_stdin() {
        let asker = CodexAsker::new("gpt-6-luna");
        let args = asker
            .args(Path::new("forma.json"), Path::new("out.json"), &[])
            .into_iter()
            .map(|a| a.to_string_lossy().to_string())
            .collect::<Vec<_>>();
        assert!(args.windows(2).any(|w| w == ["--output-schema", "forma.json"]));
        assert!(args.windows(2).any(|w| w == ["--sandbox", "read-only"]));
        assert!(args.windows(2).any(|w| w == ["-a", "never"]));
        assert_eq!(args.last().unwrap(), "-");
        assert_eq!(asker.engine_id(), "codex-cli:gpt-6-luna");
        assert_eq!(CodexAsker::new(" ").engine_id(), "codex-cli");
        assert!(!args.iter().any(|a| a == "-c"));
    }

    #[test]
    fn a_question_with_images_shows_them_first_and_a_missing_image_is_never_sent() {
        let asker = CodexAsker::new("m");
        let images = [PathBuf::from("/a/uno.jpg"), PathBuf::from("/a/dos.jpg")];
        let args = asker
            .args(Path::new("forma.json"), Path::new("out.json"), &images)
            .into_iter()
            .map(|a| a.to_string_lossy().to_string())
            .collect::<Vec<_>>();
        let at = args.iter().position(|a| a == "-i").unwrap();
        assert_eq!(&args[at - 1], "exec");
        assert_eq!(args[at + 1..at + 3].to_vec(), images.iter().map(|i| i.to_string_lossy().to_string()).collect::<Vec<_>>());
        assert!(args[at + 3].starts_with("--"));
        assert!(asker.sees_images());
        let missing = asker.ask_with_images("p", &serde_json::json!({}), &[PathBuf::from("no-esta.jpg")]);
        assert!(missing.unwrap_err().contains("cannot be shown"));
    }

    #[test]
    fn a_question_carries_the_reasoning_level_to_codex() {
        let asker = CodexAsker::new("gpt-6-luna").with_reasoning("high");
        let args = asker
            .args(Path::new("forma.json"), Path::new("out.json"), &[])
            .into_iter()
            .map(|a| a.to_string_lossy().to_string())
            .collect::<Vec<_>>();
        assert!(args.windows(2).any(|w| w == ["-c", "model_reasoning_effort=high"]));
        assert_eq!(args.last().unwrap(), "-");
        assert_eq!(asker.engine_id(), "codex-cli:gpt-6-luna@high");
    }

    #[test]
    fn model_flag_is_omitted_when_no_model_is_configured() {
        let recognizer = CodexRecognizer::new("");
        let args = recognizer.args(Path::new("a.png"), Path::new("o.txt"));
        assert!(!args.iter().any(|a| a == "--model"));
        assert_eq!(recognizer.engine_id(), "codex-cli");
    }

    /// Talks to the real Codex CLI with the machine's own login, so it
    /// only runs on request:
    /// `cargo test --lib recognition_codex -- --ignored --nocapture`
    #[test]
    #[ignore = "uses the installed Codex CLI and its subscription"]
    fn real_codex_transcribes_the_sample_page() {
        let image = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src/source_volume/fixtures/pagina-prueba.png");
        let model = std::env::var("RECOGNITION_TEST_MODEL").unwrap_or_default();
        let text = CodexRecognizer::new(&model).recognize(&image).unwrap();
        println!("{text}");
        assert!(text.contains("CONTRATO DE PRUEBA"));
        assert!(text.contains("1.284.650,75"));
        assert!(text.contains("30-71234567-9"));
    }
}
