//! Measures the text and the prints of each page (D-560, D-534) over real
//! folders, against a copy of a base read before and the answer sheet.
//!
//! Not part of the normal test run. The folders and the copy are only
//! read; the new base goes to a scratch folder removed at the end. It
//! prints numbers only, never names nor texts (R-24).
//!
//! ```text
//! MICELYA_DOCBASE_CHECK="C:\casos" MICELYA_DOCBASE_COPY="C:\tmp\copia.sqlite3" \
//! MICELYA_SIMILAR_ANSWERS="C:\hoja\respuestas.json" \
//!     cargo test --lib docbase::similar::check_pages -- --ignored --nocapture --test-threads=1
//! ```
//!
//! - Pages: the rows of `resource_page` and `resource_text`, against the
//!   pages of the fichas of shape of the copy.
//! - The text of each document, by file: the same `text_hash` and the same
//!   bytes as in the copy. The texts the copy got from a recognizer are not
//!   read here (no model): they are counted apart.
//! - The perceptual hash against the sheet: pages alike (`ALIKE_BITS` or
//!   fewer different bits), leaving out the pages alike in
//!   `COMMON_IN_FILES` files or more.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::Path;
use std::time::Instant;

use rusqlite::{Connection, OpenFlags};
use serde::Deserialize;

use crate::docbase::testing::TempDir;
use crate::docbase::{DocBase, PRIVATE_DB_FILE};
use crate::source_volume::page_print::{distance, ALIKE_BITS};
use crate::source_volume::page_shape::{profile_pdf, PdfShapes};

/// A page alike in this many files or more is a common one (a cover, a
/// form's blank page): it tells nothing.
const COMMON_IN_FILES: usize = 5;

#[derive(Deserialize)]
struct Sheet {
    root: String,
    #[serde(default)]
    pairs: Vec<Pair>,
}

#[derive(Deserialize)]
struct Pair {
    a: String,
    b: String,
    relation: String,
}

fn key(path: &str) -> String {
    path.replace('\\', "/").trim_end_matches('/').to_lowercase()
}

/// Per file hash: its text's hash, its text and the engine.
fn texts(conn: &Connection) -> HashMap<String, (Option<String>, Option<String>, Option<String>)> {
    let mut statement = conn
        .prepare(
            "SELECT f.hash, c.text_hash, d.body, d.engine FROM file f JOIN content c ON c.id = f.content_id
             LEFT JOIN derivative d ON d.content_id = c.id AND d.kind = 'text'",
        )
        .unwrap();
    let rows = statement
        .query_map([], |row| Ok((row.get::<_, String>(0)?, (row.get(1)?, row.get(2)?, row.get(3)?))))
        .unwrap();
    rows.map(|row| row.unwrap()).collect()
}

fn count(conn: &Connection, sql: &str) -> i64 {
    conn.query_row(sql, [], |row| row.get(0)).unwrap()
}

#[test]
#[ignore]
fn pages_and_their_prints() {
    let (Ok(folder), Ok(copy)) = (std::env::var("MICELYA_DOCBASE_CHECK"), std::env::var("MICELYA_DOCBASE_COPY")) else {
        println!("MICELYA_DOCBASE_CHECK and MICELYA_DOCBASE_COPY are needed");
        return;
    };
    let scratch = TempDir::new();
    let base = DocBase::open(&scratch.path().join(PRIVATE_DB_FILE)).unwrap();
    let origin = base.add_origin("1", Path::new(&folder), &[]).unwrap();
    base.scan_origin(&origin.id).unwrap();
    let started = Instant::now();
    while base.hash_pending(32).unwrap() > 0 {}
    while base.extract_pending(8).unwrap() > 0 {}
    base.purge_orphans().unwrap();
    println!("reading: {:.1}s", started.elapsed().as_secs_f32());
    let started = Instant::now();
    while base.profile_pending(16).unwrap() > 0 {}
    println!("ficha of shape with the prints: {:.1}s", started.elapsed().as_secs_f32());

    let conn = base.conn();
    // The ficha alone, over the same files, to tell what the prints cost.
    let pdfs: Vec<String> = conn
        .prepare(
            "SELECT o.location || '/' || min(l.rel_path) FROM location l JOIN origin o ON o.id = l.origin_id
             WHERE l.file_hash IN (SELECT resource_hash FROM resource_page) GROUP BY l.file_hash",
        )
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .map(|row| row.unwrap())
        .collect();
    let started = Instant::now();
    for pdf in &pdfs {
        let _ = profile_pdf(Path::new(pdf));
    }
    println!("ficha alone over the {} files with prints: {:.1}s", pdfs.len(), started.elapsed().as_secs_f32());

    let old = Connection::open_with_flags(&copy, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let mut ficha_pages = 0;
    let mut fichas = 0;
    let mut statement = old.prepare("SELECT body FROM derivative WHERE kind = 'page-shapes'").unwrap();
    for body in statement.query_map([], |row| row.get::<_, String>(0)).unwrap() {
        if let Ok(shapes) = serde_json::from_str::<PdfShapes>(&body.unwrap()) {
            fichas += 1;
            ficha_pages += shapes.pages.len();
        }
    }
    println!("copy: {ficha_pages} pages in {fichas} fichas");
    println!(
        "new: {} fichas; resource_page {} rows of {} files; resource_text {} rows of {} files ({} of extraction)",
        count(&conn, "SELECT count(*) FROM derivative WHERE kind = 'page-shapes'"),
        count(&conn, "SELECT count(*) FROM resource_page"),
        count(&conn, "SELECT count(DISTINCT resource_hash) FROM resource_page"),
        count(&conn, "SELECT count(*) FROM resource_text"),
        count(&conn, "SELECT count(DISTINCT resource_hash) FROM resource_text"),
        count(&conn, "SELECT count(*) FROM resource_text WHERE engine = 'extraction'"),
    );
    println!(
        "new: pages of PDFs in resource_text {}; files with text and no pages {}",
        count(
            &conn,
            "SELECT count(*) FROM resource_text t WHERE EXISTS (SELECT 1 FROM location l WHERE l.file_hash = t.resource_hash AND lower(l.ext) = 'pdf')"
        ),
        count(
            &conn,
            "SELECT count(*) FROM file f JOIN derivative d ON d.content_id = f.content_id AND d.kind = 'text'
             WHERE NOT EXISTS (SELECT 1 FROM resource_text t WHERE t.resource_hash = f.hash)"
        ),
    );

    // The text of each document, by file.
    let (now, before) = (texts(&conn), texts(&old));
    let (mut same_hash, mut other_hash, mut same_bytes, mut other_bytes, mut recognized, mut recognized_unread, mut only_before) =
        (0, 0, 0, 0, 0, 0, 0);
    for (hash, (text_hash, body, engine)) in &before {
        let Some(body) = body else { continue };
        if engine.as_deref() != Some("extraction") {
            recognized += 1;
            if now.get(hash).is_some_and(|(_, body, _)| body.is_none()) {
                recognized_unread += 1;
            }
            continue;
        }
        let Some((new_hash, new_body, _)) = now.get(hash) else {
            only_before += 1;
            continue;
        };
        if new_hash == text_hash { same_hash += 1 } else { other_hash += 1 }
        if new_body.as_ref() == Some(body) { same_bytes += 1 } else { other_bytes += 1 }
    }
    println!(
        "texts extracted before: same text_hash {same_hash}, other {other_hash}; same bytes {same_bytes}, other {other_bytes}; files not in the new base {only_before}"
    );
    println!("texts recognized before: {recognized}, without text here {recognized_unread}");

    // The perceptual hash against the sheet.
    let Some(sheet) = std::env::var("MICELYA_SIMILAR_ANSWERS")
        .ok()
        .and_then(|path| std::fs::read_to_string(path).ok())
        .and_then(|text| serde_json::from_str::<Sheet>(&text).ok())
    else {
        println!("no answer sheet");
        return;
    };
    let mut by_path: HashMap<String, String> = HashMap::new();
    let mut statement = conn
        .prepare("SELECT o.location, l.rel_path, l.file_hash FROM location l JOIN origin o ON o.id = l.origin_id WHERE l.file_hash IS NOT NULL")
        .unwrap();
    for row in statement.query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?))).unwrap() {
        let (root, rel, hash) = row.unwrap();
        by_path.insert(key(&format!("{root}/{rel}")), hash);
    }
    let mut prints: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut statement = conn.prepare("SELECT resource_hash, phash FROM resource_page ORDER BY resource_hash, page_no").unwrap();
    for row in statement.query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))).unwrap() {
        let (hash, phash) = row.unwrap();
        prints.entry(hash).or_default().push(phash);
    }
    // To compare by hand with another program, outside the repository.
    if let Ok(out) = std::env::var("MICELYA_PRINTS_OUT") {
        let lines: String = prints
            .iter()
            .flat_map(|(hash, pages)| pages.iter().enumerate().map(move |(index, p)| format!("{hash}	{}	{p}
", index + 1)))
            .collect();
        std::fs::write(out, lines).unwrap();
    }
    // A page is common when pages alike are in `COMMON_IN_FILES` files.
    let all: Vec<(&String, &String)> = prints.iter().flat_map(|(hash, pages)| pages.iter().map(move |p| (hash, p))).collect();
    let common: BTreeSet<String> = all
        .iter()
        .filter(|(_, phash)| {
            let files: BTreeSet<&String> =
                all.iter().filter(|(_, other)| distance(phash, other).is_some_and(|d| d <= ALIKE_BITS)).map(|(hash, _)| *hash).collect();
            files.len() >= COMMON_IN_FILES
        })
        .map(|(_, phash)| (*phash).clone())
        .collect();
    println!("pages with prints {}, common ones {}", all.len(), common.len());
    // As the measurement in the cloud: of the shorter file A against B,
    // each page of A (a common one never counts) alike a page of B: all of
    // them, the same (as many pages) or inside (B longer); some, partial.
    let relation = |a: &str, b: &str| -> &'static str {
        let (Some(a), Some(b)) = (prints.get(a), prints.get(b)) else { return "none" };
        let (a, b) = if a.len() > b.len() { (b, a) } else { (a, b) };
        let hits = a
            .iter()
            .filter(|page| !common.contains(*page) && b.iter().any(|other| distance(page, other).is_some_and(|d| d <= ALIKE_BITS)))
            .count();
        match hits {
            0 => "none",
            n if n == a.len() && a.len() == b.len() => "same",
            n if n == a.len() => "inside",
            _ => "partial",
        }
    };
    let whole = |a: &str, b: &str| matches!(relation(a, b), "same" | "inside");
    // The relations of the sheet that are the same document, and the ones
    // that are not left out of the false ones (a signed copy, a version).
    let positive = ["mismo-documento", "practicamente-el-mismo", "vista-previa", "contenido-en"];
    let root = sheet.root.clone();
    let hash_of = |path: &str| by_path.get(&key(&format!("{root}/{path}"))).cloned();
    let (mut same_total, mut same_alike, mut same_identical, mut same_unprinted) = (0, 0, 0, 0);
    let (mut signed_total, mut signed_inside, mut signed_partial) = (0, 0, 0);
    let (mut others_total, mut others_alike) = (0, 0);
    for pair in &sheet.pairs {
        let (Some(a), Some(b)) = (hash_of(&pair.a), hash_of(&pair.b)) else { continue };
        let printed = prints.contains_key(&a) && prints.contains_key(&b);
        if !printed || a == b {
            if pair.relation == "mismo-documento" {
                if a == b { same_identical += 1 } else { same_unprinted += 1 }
            }
            continue;
        }
        match pair.relation.as_str() {
            "mismo-documento" => {
                same_total += 1;
                same_alike += usize::from(whole(&a, &b));
            }
            "ejemplar-firmado" => {
                signed_total += 1;
                signed_inside += usize::from(whole(&a, &b));
                signed_partial += usize::from(relation(&a, &b) == "partial");
            }
            "version" => {}
            other if !positive.contains(&other) => {
                others_total += 1;
                others_alike += usize::from(whole(&a, &b));
            }
            _ => {}
        }
    }
    println!(
        "same document: {same_alike} of {same_total} not identical are the same or inside ({same_identical} identical, {same_unprinted} without prints)"
    );
    println!("signed copy: the original inside the signed one in {signed_inside} of {signed_total} (partial: {signed_partial})");
    println!("not the same document: {others_alike} of {others_total} pairs with prints are the same or inside");
}
