//! Measures the rules of the numbers and the names of the identity (E5 of
//! 2 nov) on a COPY of a base already read: the resolution is computed
//! again with the readings and the verdicts that copy keeps, without
//! asking the model (no asker, no budget), and the entity of each mention
//! before and after is written to a file, with the pairs whose numbers
//! meet and whose names were beyond the caps. Nothing goes back to any
//! base: `open_base` copies the copy again into a temporary folder.
//!
//! Not part of the normal test run:
//!
//! ```text
//! MICELYA_DOCBASE_COPY=…\base.sqlite3 MICELYA_NUMBERS_OUT=…\salida.json \
//!     cargo test --lib docbase::knowledge::check_numbers -- --ignored --nocapture --test-threads=1
//! ```

use std::collections::BTreeMap;

use super::check::open_base;
use super::resolve::{resolve, write, Asking, ResolveReport};

fn partition(conn: &rusqlite::Connection) -> BTreeMap<i64, (i64, String, String, Option<String>, Option<String>)> {
    let mut statement = conn
        .prepare(
            "SELECT m.id, m.version_id, m.kind, m.written, e.id, m.resolved_by FROM knowledge_mention m
             LEFT JOIN knowledge_entity e ON e.id = m.entity_id AND e.state = 'active'
             WHERE m.kind IN ('persona', 'organizacion')",
        )
        .unwrap();
    let rows = statement
        .query_map([], |row| {
            Ok((row.get::<_, i64>(0)?, (row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?)))
        })
        .unwrap();
    rows.collect::<Result<_, _>>().unwrap()
}

#[test]
#[ignore = "reads a copy of a real base"]
fn the_numbers_and_the_names_on_a_copy_of_a_base() {
    let Some((_scratch, base)) = open_base() else {
        println!("MICELYA_DOCBASE_COPY is not set: nothing to measure");
        return;
    };
    let before = partition(&base.conn());
    let engine = crate::docbase::similar::classify::ask_engine(&base.conn())
        .unwrap()
        .map(|c| crate::source_volume::asking::engine_id(&c));
    let mut report = ResolveReport::default();
    let mut asking = Asking { asker: None, engine, budget: 0, collect: None };
    let outcome = resolve(&base, &mut asking, &mut report).unwrap();
    let apart: Vec<serde_json::Value> = outcome
        .numbers_apart
        .iter()
        .map(|p| {
            let side = |i: usize| {
                let cluster = &outcome.clusters[i];
                serde_json::json!({
                    "version": cluster.content_id,
                    "kind": cluster.kind,
                    "written": outcome.rows[cluster.principal].written,
                    "mentions": cluster.rows.iter().map(|&r| outcome.rows[r].id).collect::<Vec<_>>(),
                })
            };
            serde_json::json!({
                "a": side(p.a), "b": side(p.b), "identifier": p.kind, "values": [p.values.0, p.values.1],
                "near": p.near, "letters": p.far.map(|f| f.0), "share": p.far.map(|f| f.1),
            })
        })
        .collect();
    write(&base, &outcome, &mut report).unwrap();
    let after = partition(&base.conn());
    let mentions: Vec<serde_json::Value> = before
        .iter()
        .map(|(id, (version, kind, written, old, by_old))| {
            let (_, _, _, new, by_new) = &after[id];
            serde_json::json!({
                "id": id, "version": version, "kind": kind, "written": written,
                "before": old, "after": new, "byBefore": by_old, "byAfter": by_new,
            })
        })
        .collect();
    println!("{report:?}");
    let out = serde_json::json!({ "report": format!("{report:?}"), "mentions": mentions, "apart": apart });
    let path = std::env::var("MICELYA_NUMBERS_OUT").unwrap_or_else(|_| "numbers.json".to_string());
    std::fs::write(&path, serde_json::to_string_pretty(&out).unwrap()).unwrap();
    println!("written: {path}");
}
