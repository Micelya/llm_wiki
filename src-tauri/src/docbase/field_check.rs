//! Runs the base over real folders and prints what it found.
//!
//! Not part of the normal test run: it reads whatever folders it is
//! given, which can be large or on a cloud drive. It only reads them;
//! the database goes to a scratch folder that is removed at the end.
//!
//! ```text
//! MICELYA_DOCBASE_CHECK="C:\casos;G:\Mi unidad\Otra" \
//!     cargo test --lib docbase::field_check -- --ignored --nocapture
//! ```

use std::path::Path;
use std::time::Instant;

use super::query::DocumentFilter;
use super::testing::TempDir;
use super::{DocBase, Visible, PRIVATE_DB_FILE};

#[test]
#[ignore = "reads the real folders named in MICELYA_DOCBASE_CHECK"]
fn inventory_of_real_folders() {
    let Ok(folders) = std::env::var("MICELYA_DOCBASE_CHECK") else {
        println!("MICELYA_DOCBASE_CHECK is not set: nothing to check");
        return;
    };
    let scratch = TempDir::new();
    let base = DocBase::open(&scratch.path().join(PRIVATE_DB_FILE)).unwrap();

    for (index, folder) in folders.split(';').filter(|f| !f.trim().is_empty()).enumerate() {
        let folder = Path::new(folder.trim());
        let name = format!(
            "{} {}",
            index + 1,
            folder.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default()
        );
        let origin = base.add_origin(&name, folder, &[]).unwrap();
        let started = Instant::now();
        let report = base.scan_origin(&origin.id).unwrap();
        println!(
            "\n== {} ==\n   {}\n   sweep: {} files, reachable: {} ({:.1}s)",
            name,
            folder.display(),
            report.total,
            report.reachable,
            started.elapsed().as_secs_f32()
        );
    }

    let started = Instant::now();
    while base.hash_pending(32).unwrap() > 0 {}
    println!("\nidentity: {:.1}s", started.elapsed().as_secs_f32());
    let started = Instant::now();
    while base.extract_pending(8).unwrap() > 0 {}
    println!("text: {:.1}s", started.elapsed().as_secs_f32());
    base.purge_orphans().unwrap();

    let all = DocumentFilter { limit: Some(1000), sort: Some("folder".into()), ..Default::default() };
    let page = base.documents(&Visible::All, &all).unwrap();
    let conn_count = |sql: &str| -> i64 { base.conn().query_row(sql, [], |row| row.get(0)).unwrap() };

    println!("\nlocations: {}", page.total);
    println!("files (distinct bytes): {}", conn_count("SELECT count(*) FROM file"));
    println!("documents (contents): {}", conn_count("SELECT count(*) FROM content"));
    println!(
        "contents made of more than one file (same text, other bytes): {}",
        conn_count("SELECT count(*) FROM (SELECT content_id FROM file GROUP BY content_id HAVING count(*) > 1)")
    );
    println!("words indexed: {}", conn_count("SELECT coalesce(sum(word_count), 0) FROM content"));

    let mut states: std::collections::BTreeMap<String, usize> = Default::default();
    for row in &page.rows {
        *states.entry(row.state.clone()).or_default() += 1;
    }
    println!("\nby state:");
    for (state, count) in &states {
        println!("   {state:<20} {count}");
    }

    let duplicates: Vec<_> = page.rows.iter().filter(|row| row.copies > 1).collect();
    println!("\nlocations that are copies of another: {}", duplicates.len());
    for row in duplicates.iter().take(30) {
        println!("   x{}  [{}] {}", row.copies, row.origin_name, row.rel_path);
    }

    let problems: Vec<_> = page
        .rows
        .iter()
        .filter(|row| matches!(row.state.as_str(), "error" | "unreadable" | "too-large"))
        .collect();
    println!("\nproblems: {}", problems.len());
    for row in problems.iter().take(30) {
        let detail = base.document(&Visible::All, row.location_id).unwrap().unwrap();
        println!(
            "   {}  [{}] {} — {}",
            row.state,
            row.origin_name,
            row.rel_path,
            detail.error.unwrap_or_default().chars().take(160).collect::<String>()
        );
    }
}
