//! Integrates every record followed in the portals into a copy of a real
//! base, as the screen «Portales» does when it opens, and prints only
//! numbers: the active matters before and after, those retired, the
//! questions for a person. The record files and the documents are only
//! read; the record files are not saved.
//!
//! Not part of the normal test run. Only `MICELYA_DOCBASE_COPY` (a copy of
//! the app's base, outside the repository); the records' folder is the
//! `Expedientes` folder of the origin that holds it.
//!
//! ```text
//! MICELYA_DOCBASE_COPY=… cargo test --lib portals::check_records -- --ignored --nocapture --test-threads=1
//! ```

use std::fs;

use super::store;
use crate::docbase::knowledge::check::open_base;
use crate::docbase::origins::path_inside;
use crate::docbase::Visible;

#[test]
#[ignore = "reads a copy of a real base"]
fn what_the_integration_of_the_records_does_to_the_matters() {
    let Some((_scratch, base)) = open_base() else {
        println!("MICELYA_DOCBASE_COPY is not set: nothing to print");
        return;
    };
    let count = |sql: &str| -> i64 { base.conn().query_row(sql, [], |row| row.get(0)).unwrap() };
    let active_before = count("SELECT count(*) FROM knowledge_matter WHERE state = 'active'");
    let by_person_before = count("SELECT count(*) FROM knowledge_matter WHERE state = 'active' AND name_by_person = 1");
    let questions_before = base.knowledge_attention(&Visible::All).unwrap().matters_to_join.len();
    let mut records = 0;
    let mut created = 0;
    let mut retired = 0;
    let mut questions = 0;
    let mut added = 0;
    for origin in base.origins(&Visible::All).unwrap() {
        let expedientes = origin.root().join("Expedientes");
        let Ok(portals) = fs::read_dir(&expedientes) else { continue };
        for portal in portals.flatten().filter(|e| e.path().is_dir()) {
            for (folder, record) in store::followed(&portal.path()) {
                if record.updated_at.is_none() {
                    continue;
                }
                let Some(rel) = path_inside(&origin.root(), &folder) else { continue };
                let mut record = record;
                let contents: Vec<i64> = {
                    let conn = base.conn();
                    let prefix = format!("{rel}/");
                    let mut statement = conn
                        .prepare(
                            "SELECT DISTINCT f.version_id FROM location l JOIN file f ON f.hash = l.file_hash
                             WHERE l.origin_id = ?1 AND substr(l.rel_path, 1, length(?2)) = ?2 AND l.ext = 'pdf' AND f.version_id IS NOT NULL",
                        )
                        .unwrap();
                    statement.query_map(rusqlite::params![origin.id, prefix], |row| row.get(0)).unwrap().map(Result::unwrap).collect()
                };
                let outcome = base.knowledge_integrate_record(&store::record_input(&record), record.matter_id.as_deref(), &contents).unwrap();
                record.matter_id = Some(outcome.matter_id.clone());
                records += 1;
                created += usize::from(outcome.created);
                retired += outcome.retired.len();
                questions += outcome.questions;
                added += outcome.added;
            }
        }
    }
    let active_after = count("SELECT count(*) FROM knowledge_matter WHERE state = 'active'");
    let by_person_after = count("SELECT count(*) FROM knowledge_matter WHERE state = 'active' AND name_by_person = 1");
    let attention = base.knowledge_attention(&Visible::All).unwrap();
    println!("records integrated: {records}");
    println!("active matters: {active_before} -> {active_after}");
    println!("named by a person (active): {by_person_before} -> {by_person_after}");
    println!("matters made: {created}, retired: {retired}, documents put in: {added}");
    println!("new questions: {questions}; questions in Atencion: {questions_before} -> {}", attention.matters_to_join.len());
    for question in &attention.matters_to_join {
        println!(
            "  question: shared {}, citing {}, other origin {}, records {}",
            question.shared,
            question.citing,
            question.other_origin,
            question.records.len()
        );
    }
    println!("records kept as data of a matter: {}", count("SELECT count(*) FROM knowledge_matter_record"));
}
