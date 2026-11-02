//! Looking writes nothing (etapa P, the check the coordinator asked for at
//! 10:11; audit 1A, row 2.6): every order a screen uses only to look, from
//! `commands.rs`, is called on a base with documents, readings, entities
//! and pages, and the base is compared whole before and after each one,
//! table by table and row by row, with the count of changes of its
//! connection and the version of its shape. An order that wrote would be
//! work of the process, not of the screen.
//!
//! Every text and answer is invented (R-24): a simulated model answers.

use std::collections::BTreeMap;

use rusqlite::types::Value;
use rusqlite::Connection;

use super::knowledge::extract::RereadTarget;
use super::knowledge::tests::{answer, base_with, claim, empty_answer, is_knowledge, mention, type_answer, FakeModel};
use super::progress::{Scope, STAGES};
use super::query::DocumentFilter;
use super::{DocBase, Visible};

const LETTER: &str = "CARTA DE PRUEBA\n\nLa empresa Ejemplo Cuatro S.A. comunica a Persona Cinco que el contrato inventado \
vence el 1 de abril de 2020.\n\nPersona Cinco constituye domicilio en Calle Inventada 789.";
const NOTE: &str = "NOTA INTERNA\n\nUna nota inventada, sin nombres, para que la base tenga otro documento.";

fn letter_answer() -> String {
    answer(
        vec![
            mention("a", "organizacion", "Ejemplo Cuatro S.A.", "remitente", "La empresa Ejemplo Cuatro S.A. comunica", serde_json::json!([])),
            mention("b", "persona", "Persona Cinco", "destinatario", "comunica a Persona Cinco", serde_json::json!([])),
        ],
        vec![claim("b", "domicilio", None, Some("Calle Inventada 789"), "Persona Cinco constituye domicilio en Calle Inventada 789", None)],
    )
}

/// Everything the base holds: each table with its rows, sorted, the count
/// of changes of the connection and the version of its shape.
fn whole(conn: &Connection) -> (BTreeMap<String, Vec<String>>, i64, i64) {
    let tables: Vec<String> = {
        let mut statement = conn.prepare("SELECT name FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite\\_%' ESCAPE '\\' ORDER BY name").unwrap();
        let rows = statement.query_map([], |row| row.get(0)).unwrap();
        rows.collect::<Result<_, _>>().unwrap()
    };
    let mut out = BTreeMap::new();
    for table in tables {
        let mut statement = conn.prepare(&format!("SELECT * FROM \"{table}\"")).unwrap();
        let columns = statement.column_count();
        let rows = statement
            .query_map([], |row| (0..columns).map(|i| row.get::<_, Value>(i)).collect::<Result<Vec<_>, _>>())
            .unwrap();
        let mut rows: Vec<String> = rows.map(|row| format!("{:?}", row.unwrap())).collect();
        rows.sort();
        out.insert(table, rows);
    }
    let changes = conn.query_row("SELECT total_changes()", [], |row| row.get(0)).unwrap();
    let shape = conn.query_row("PRAGMA schema_version", [], |row| row.get(0)).unwrap();
    (out, changes, shape)
}

/// Calls `look` and says whether the base is the same after it.
fn same_after(base: &DocBase, order: &str, look: impl FnOnce()) -> Option<String> {
    let before = whole(&base.conn());
    look();
    let after = whole(&base.conn());
    if before == after {
        return None;
    }
    let tables: Vec<&String> = before.0.iter().filter(|(t, rows)| after.0.get(*t) != Some(rows)).map(|(t, _)| t).collect();
    Some(format!("{order} wrote: tables {tables:?}, changes {} → {}, shape {} → {}", before.1, after.1, before.2, after.2))
}

/// A base with what the screens look at: two documents read, typed and
/// read for entities, their identity resolved and their pages written.
fn looked_at() -> (crate::docbase::testing::TempDir, DocBase) {
    let (dir, base) = base_with(&[("carta.txt", LETTER), ("nota.txt", NOTE)]);
    let asker = FakeModel::new("falso:1", |prompt| {
        if !is_knowledge(prompt) {
            type_answer("carta-documento")
        } else if prompt.contains("CARTA DE PRUEBA") {
            letter_answer()
        } else {
            empty_answer()
        }
    });
    base.types_run(&asker, 50).unwrap();
    let read = base.knowledge_run(&asker, 50).unwrap();
    assert_eq!(read.extracted, 2, "{read:?}");
    base.knowledge_resolve(Some(&asker), 50).unwrap();
    base.knowledge_project(Some(&asker), 50).unwrap();
    (dir, base)
}

#[test]
fn every_order_of_the_screens_that_looks_writes_nothing() {
    let (_dir, base) = looked_at();
    let visible = Visible::All;
    let documents = base.documents(&visible, &DocumentFilter::default()).unwrap();
    assert_eq!(documents.rows.len(), 2, "the base has its two documents");
    let locations: Vec<i64> = documents.rows.iter().map(|r| r.location_id).collect();
    let versions: Vec<i64> = locations
        .iter()
        .filter_map(|l| base.document(&visible, *l).unwrap().and_then(|d| d.row.content_id))
        .collect();
    let origin = base.origin_summaries(&visible).unwrap()[0].origin.id.clone();
    let entities: Vec<String> = base.knowledge_entities(&visible, None, None).unwrap().into_iter().map(|e| e.id).collect();
    assert!(!entities.is_empty(), "the letter names people and organizations");
    let hashes: Vec<String> = base
        .conn()
        .prepare("SELECT hash FROM file")
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();

    let mut wrote: Vec<String> = Vec::new();
    let mut look = |order: &str, call: &dyn Fn()| {
        if let Some(why) = same_after(&base, order, call) {
            wrote.push(why);
        }
    };

    look("docbase_data_status", &|| drop(base.schema_version()));
    look("docbase_list_origins", &|| drop(base.origin_summaries(&visible)));
    look("docbase_home", &|| {
        drop(base.health(&visible));
        drop(base.origin_summaries(&visible));
        drop(base.activity(&visible, 12));
    });
    look("docbase_list_documents", &|| drop(base.documents(&visible, &DocumentFilter::default())));
    look("docbase_label_groups", &|| drop(base.label_groups()));
    look("docbase_label_rules", &|| drop(base.label_rules(&origin)));
    look("docbase_list_folder", &|| drop(base.list_folder(&visible, &origin, "")));
    look("docbase_similar_pending", &|| drop(base.similar_pending(&visible)));
    look("docbase_similar_groups", &|| drop(base.similar_groups(&visible)));
    look("docbase_templates", &|| drop(base.templates(&visible)));
    look("docbase_types_plan", &|| drop(base.types_plan()));
    look("docbase_proposed_types", &|| drop(base.proposed_types()));
    look("docbase_document_types", &|| drop(base.decided_types()));
    look("docbase_process_state", &|| drop(base.process_view()));
    look("docbase_ai_services", &|| drop(base.ai_services()));
    look("docbase_model_questions", &|| {
        for question in super::questions::model_questions(Some(&base), None).unwrap_or_default() {
            drop(base.last_model_question(&question.id));
        }
    });
    look("docbase_knowledge_status", &|| drop(base.knowledge_status(&visible)));
    look("docbase_graph", &|| drop(base.graph(&visible)));
    look("docbase_knowledge_plan", &|| drop(base.knowledge_plan()));
    look("docbase_knowledge_entities", &|| drop(base.knowledge_entities(&visible, None, None)));
    look("docbase_knowledge_orphan_notes", &|| drop(base.knowledge_orphan_notes()));
    look("docbase_search", &|| drop(crate::commands::search::search_base(&base, "Persona Cinco", 20)));
    look("docbase_knowledge_overview", &|| drop(base.knowledge_overview()));
    look("docbase_saved_queries", &|| drop(base.saved_queries()));
    look("docbase_research_loose", &|| drop(base.loose_research()));
    look("docbase_background_reading", &|| drop(base.background_reading()));
    look("docbase_progress", &|| drop(base.progress(&Scope::default())));
    look("docbase_progress_items", &|| {
        for stage in STAGES {
            drop(base.progress_items(stage, &Scope::default(), 50));
        }
    });
    look("docbase_progress_folders", &|| drop(base.progress_folders(&origin)));
    look("docbase_knowledge_doubts", &|| drop(base.knowledge_doubts(&visible)));
    look("docbase_knowledge_decisions", &|| drop(base.knowledge_decisions()));
    look("docbase_knowledge_reread_plan", &|| drop(base.knowledge_reread_plan(&RereadTarget::default())));
    look("docbase_knowledge_attention", &|| drop(base.knowledge_attention(&visible)));
    look("docbase_failures", &|| drop(base.failures_open()));
    look("docbase_thresholds", &|| drop(base.thresholds()));
    look("docbase_knowledge_matters", &|| drop(base.knowledge_matters(&visible)));
    look("docbase_reading_quality", &|| drop(base.reading_quality()));
    look("docbase_confidence_effect", &|| drop(base.confidence_effect(0.2, 0.9)));
    for location in &locations {
        look("docbase_get_document", &|| drop(base.document(&visible, *location)));
        look("docbase_document_copies", &|| drop(base.copies(&visible, *location)));
        look("docbase_similar_documents", &|| drop(base.similar_of(&visible, *location)));
        look("docbase_similar_template", &|| drop(base.similar_template(&visible, *location)));
        look("docbase_document_type", &|| {
            drop(base.document_type_of(*location));
            drop(base.document_seal(*location));
        });
        look("docbase_knowledge_document", &|| drop(base.knowledge_of(&visible, *location)));
        look("docbase_knowledge_document_entities", &|| drop(base.knowledge_document_entities(&visible, *location)));
        for version in &versions {
            look("docbase_get_document_part", &|| drop(base.document_part(&visible, *location, *version)));
        }
    }
    for version in &versions {
        look("docbase_knowledge_document_article", &|| drop(base.knowledge_document_article(*version)));
        look("docbase_ai_reading_cost", &|| drop(base.ai_reading_cost(*version)));
    }
    for hash in &hashes {
        look("docbase_included_parts", &|| drop(base.strip_parts(hash)));
    }
    let mut paths: Vec<String> = Vec::new();
    for id in &entities {
        look("docbase_knowledge_entity", &|| drop(base.knowledge_entity(&visible, id)));
        look("docbase_knowledge_article", &|| drop(base.knowledge_article(id)));
        look("docbase_knowledge_notes", &|| drop(base.knowledge_notes(id)));
        look("docbase_research_of", &|| drop(base.research_of(id)));
        if let Ok(Some(article)) = base.knowledge_article(id) {
            paths.push(article.path);
        }
    }
    for matter in base.knowledge_matters(&visible).unwrap() {
        look("docbase_knowledge_matter", &|| drop(base.knowledge_matter(&visible, &matter.id)));
    }
    assert!(!paths.is_empty(), "the entities have their pages");
    for path in &paths {
        look("docbase_knowledge_object_of_page", &|| drop(base.knowledge_object_of_page(path)));
        look("docbase_knowledge_page_text", &|| {
            if let Ok(Some(object)) = base.knowledge_object_of_page(path) {
                drop(base.knowledge_page_text(&object.path));
            }
        });
    }
    assert!(wrote.is_empty(), "looking wrote in the base:\n{}", wrote.join("\n"));
    // The comparison sees a write: keeping a threshold is one.
    let kept = same_after(&base, "docbase_set_thresholds", || {
        base.set_thresholds(&super::thresholds::Thresholds { quality: 0.4, ..Default::default() }).unwrap()
    });
    assert!(kept.is_some_and(|why| why.contains("docbase_setting")), "the comparison has to see a write");
}
