//! Diagnoses, on a COPY of a base, a step that counts work its operation
//! does not take (the small test of 2 nov stopped in the identity with 96
//! pending): the count of every step, and for the identity the questions a
//! resolution would ask now, each with whether a failure holds it and why.
//! Asks nothing: the questions are only collected. Then, with a stand-in
//! model that must not be called (it panics), it runs the identity step of
//! the loop as the app would, and says how the step stands after it.
//! Nothing goes back to any base: `open_base` copies the copy again into a
//! temporary folder.
//!
//! Not part of the normal test run:
//!
//! ```text
//! MICELYA_DOCBASE_COPY=…\base.sqlite3 \
//!     cargo test --lib docbase::process::check_stuck -- --ignored --nocapture --test-threads=1
//! ```

use std::collections::BTreeMap;

use crate::docbase::knowledge::check::open_base;
use crate::docbase::knowledge::resolve::{resolve, Asking, ResolveReport};
use crate::source_volume::asking::DocumentAsker;

use super::{Scope, Step, STEPS};

/// A model that must not be asked: every question of the identity left
/// in the copy is held, or the diagnosis is wrong.
/// It says it is the engine of the base, so the verdicts kept count.
struct NotToBeAsked(String);

impl DocumentAsker for NotToBeAsked {
    fn engine_id(&self) -> String {
        self.0.clone()
    }

    fn ask(&self, prompt: &str, _schema: &serde_json::Value) -> Result<String, String> {
        panic!("the diagnosis asked the model: {}", prompt.chars().take(200).collect::<String>())
    }
}

fn print_steps(base: &crate::docbase::DocBase, title: &str) {
    println!("== {title}");
    let state = base.process_state(&Scope::All, false).unwrap();
    for step in STEPS {
        let count = state.step(step).unwrap().count;
        if count.pending + count.set_aside > 0 {
            println!(
                "paso {:>2} {:<28} pendientes {:>4} apartados {:>4} (persona {}, confianza baja {})",
                step.number(),
                step.about().name,
                count.pending,
                count.set_aside,
                count.person,
                count.low
            );
        }
    }
    println!("primer paso incompleto: {:?}", state.first_incomplete.map(Step::number));
}

#[test]
#[ignore = "reads a copy of a real base"]
fn the_steps_that_count_work_their_operation_does_not_take_on_a_copy_of_a_base() {
    let Some((_scratch, base)) = open_base() else {
        println!("MICELYA_DOCBASE_COPY is not set: nothing to diagnose");
        return;
    };
    print_steps(&base, "la copia, como quedó");
    let marks: Vec<(String, String)> = {
        let conn = base.conn();
        let mut statement = conn.prepare("SELECT scope, ref FROM knowledge_dirty ORDER BY scope, ref").unwrap();
        let rows = statement.query_map([], |row| Ok((row.get(0)?, row.get(1)?))).unwrap();
        rows.collect::<Result<_, _>>().unwrap()
    };
    let mut by_scope: BTreeMap<&str, usize> = BTreeMap::new();
    for (scope, _) in &marks {
        *by_scope.entry(scope.as_str()).or_default() += 1;
    }
    println!("marcas de la capa de conocimiento: {by_scope:?}");

    // The questions a resolution would ask now, collected without asking.
    let engine = crate::docbase::similar::classify::ask_engine(&base.conn())
        .unwrap()
        .map(|c| crate::source_volume::asking::engine_id(&c));
    let mut report = ResolveReport::default();
    let mut asking = Asking { asker: None, engine: engine.clone(), budget: 0, collect: Some(Vec::new()) };
    resolve(&base, &mut asking, &mut report).unwrap();
    let questions = asking.collect.unwrap_or_default();
    let low = super::steps::low_confidence(&base.conn()).unwrap();
    let failures: BTreeMap<i64, Vec<String>> = {
        let conn = base.conn();
        let mut statement = conn
            .prepare("SELECT CAST(item AS INTEGER), stage || ' (' || state || ')' FROM failure WHERE item_kind = 'version' AND state <> 'resolved'")
            .unwrap();
        let rows = statement.query_map([], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))).unwrap();
        let mut out: BTreeMap<i64, Vec<String>> = BTreeMap::new();
        for row in rows {
            let (version, what) = row.unwrap();
            out.entry(version).or_default().push(what);
        }
        out
    };
    let mut held = 0;
    println!("== las preguntas de identidad que quedan: {} (motor {:?})", questions.len(), engine);
    for question in &questions {
        let is_held = base.question_held(question.task, question.content_id, &question.key);
        held += usize::from(is_held);
        println!(
            "documento {:>4} retenida {:<5} confianza baja {:<5} fallas {:?}",
            question.content_id,
            is_held,
            low.contains(&question.content_id),
            failures.get(&question.content_id).cloned().unwrap_or_default()
        );
    }
    println!("retenidas por una falla: {held} de {}", questions.len());

    // The identity step as the loop does it, with a model that must not
    // be asked.
    let model = NotToBeAsked(engine.clone().unwrap_or_default());
    let report = base.knowledge_resolve(Some(&model), 1).unwrap();
    println!(
        "una operación de identidad: preguntó {}, cambió {}, falló {}, quedan {}, retenidas por una falla {} (documentos {:?})",
        report.asked, report.changed, report.failed, report.left, report.held, report.held_documents
    );
    print_steps(&base, "después de una operación de identidad");
}
