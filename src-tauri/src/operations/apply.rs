//! Applying what a person confirmed, and undoing it (11.4). Each change is
//! made by the same function of the base the screen's button uses, so it
//! is what a person would have done by hand: a decision of a person, firm
//! and reversible, that no computation overwrites (D-174). What undoing it
//! needs is returned as JSON and kept in the record.

use serde_json::{json, Value};

use crate::docbase::knowledge::decide::IdentityDecision;
use crate::docbase::knowledge::matter_edit::MatterSnapshot;
use crate::docbase::DocBase;

use super::card::Card;

fn contents(keys: &[String]) -> Vec<i64> {
    keys.iter().filter_map(|k| k.strip_prefix("doc-").and_then(|n| n.parse().ok())).collect()
}

fn matter_of(card: &Card) -> Result<String, String> {
    card.target
        .as_ref()
        .and_then(|t| t.key.clone())
        .ok_or_else(|| "Falta el caso".to_string())
}

fn status_of_word(word: &str) -> &'static str {
    match word {
        "cerrado" => "closed",
        "archivado" => "archived",
        _ => "open",
    }
}

/// The documents of a matter the model proposed and nobody answered.
fn proposed_in(base: &DocBase, matter: &str) -> Result<Vec<i64>, String> {
    let conn = base.conn();
    let proposals = crate::docbase::knowledge::matters::proposals(&conn)?;
    Ok(proposals.into_iter().filter(|(_, list)| list.iter().any(|(m, _)| m == matter)).map(|(c, _)| c).collect())
}

/// Applies a confirmed card to the things ticked (`keys`). `note`: who
/// asked for it, for the decisions it makes. Returns what undoing it needs.
pub fn apply(base: &DocBase, card: &Card, keys: &[String], note: &str) -> Result<Value, String> {
    let input = &card.input;
    let text = |key: &str| input.get(key).and_then(Value::as_str).unwrap_or_default().trim().to_string();
    match card.operation.as_str() {
        "caso_agregar_documentos" => {
            let matter = matter_of(card)?;
            let docs = contents(keys);
            if docs.is_empty() {
                return Err("No quedó ningún documento tildado".to_string());
            }
            let before = base.knowledge_matter_snapshot(&matter, &docs)?;
            for content in &docs {
                if let Err(error) = base.knowledge_choose_matter_document(&matter, &format!("content:{content}"), "in") {
                    // What was done goes back: all or nothing.
                    let _ = base.knowledge_restore_matter(&before, false);
                    return Err(error);
                }
            }
            Ok(json!({ "snapshot": before }))
        }
        "caso_sacar_documentos" => {
            let matter = matter_of(card)?;
            let docs = contents(keys);
            if docs.is_empty() {
                return Err("No quedó ningún documento tildado".to_string());
            }
            let proposed = proposed_in(base, &matter)?;
            let (leave, out): (Vec<i64>, Vec<i64>) = docs.iter().partition(|c| proposed.contains(c));
            // A proposed matter keeps one document at least.
            let before = base.knowledge_matter_snapshot(&matter, &docs)?;
            let firm_left = before.documents.iter().filter(|d| !out.contains(&d.content_id)).count();
            if before.origin == "proposed" && firm_left == 0 && !out.is_empty() {
                return Err("A un caso propuesto no se le puede sacar el último documento: para que no quede, descartalo.".to_string());
            }
            let mut decisions: Vec<i64> = Vec::new();
            // The model's proposals first, while the matter still has its
            // documents to keep them apart from.
            for content in &leave {
                match base.knowledge_leave_matter(&format!("content:{content}"), &matter, Some(note)) {
                    Ok(ids) => decisions.extend(ids),
                    Err(error) => {
                        for id in &decisions {
                            let _ = base.knowledge_undo(*id);
                        }
                        return Err(error);
                    }
                }
            }
            for content in &out {
                if let Err(error) = base.knowledge_choose_matter_document(&matter, &format!("content:{content}"), "out") {
                    let _ = base.knowledge_restore_matter(&before, false);
                    for id in &decisions {
                        let _ = base.knowledge_undo(*id);
                    }
                    return Err(error);
                }
            }
            Ok(json!({ "snapshot": before, "decisions": decisions }))
        }
        "caso_crear" => {
            let name = card.target.as_ref().and_then(|t| t.name.clone()).unwrap_or_else(|| text("nombre"));
            let documents: Vec<String> = contents(keys).iter().map(|c| format!("content:{c}")).collect();
            let id = base.knowledge_create_matter(&name, &documents)?;
            Ok(json!({ "created": id }))
        }
        "caso_renombrar" => {
            let matter = matter_of(card)?;
            let before = base.knowledge_matter_snapshot(&matter, &[])?;
            base.knowledge_rename_matter(&matter, &text("nombre"))?;
            Ok(json!({ "snapshot": before }))
        }
        "caso_decidir" => {
            let matter = matter_of(card)?;
            let before = base.knowledge_matter_snapshot(&matter, &[])?;
            let decision = match text("decision").as_str() {
                "confirmar" => "confirm",
                "descartar" => "discard",
                _ => "reopen",
            };
            base.knowledge_decide_matter(&matter, decision)?;
            Ok(json!({ "snapshot": before }))
        }
        "caso_cambiar_estado" => {
            let matter = matter_of(card)?;
            let before = base.knowledge_matter_snapshot(&matter, &[])?;
            base.knowledge_set_matter_status(&matter, status_of_word(&text("estado")))?;
            Ok(json!({ "status": before.status, "matter": matter }))
        }
        "fichas_unir" | "fichas_separar" => {
            let entities: Vec<String> = card.items.iter().filter_map(|i| i.key.clone()).filter(|k| k.starts_with("ent-")).collect();
            let document = card.items.iter().filter_map(|i| i.key.clone()).find(|k| k.starts_with("doc-"));
            let shown = json!({
                "reason": note,
                "sides": card.items.iter().map(|i| i.detail.clone().unwrap_or_else(|| json!({ "key": i.key, "name": i.name }))).collect::<Vec<_>>(),
            })
            .to_string();
            let ids = match (card.operation.as_str(), document) {
                ("fichas_separar", Some(document)) => {
                    let entity = entities.first().cloned().ok_or_else(|| "Falta la ficha".to_string())?;
                    let content: i64 = document.trim_start_matches("doc-").parse().map_err(|_| "Falta el documento".to_string())?;
                    let mentions: Vec<i64> = {
                        let conn = base.conn();
                        let mut statement = conn
                            .prepare("SELECT id FROM knowledge_mention WHERE entity_id = ?1 AND content_id = ?2 ORDER BY id")
                            .map_err(crate::docbase::sql_err)?;
                        let rows = statement.query_map(rusqlite::params![entity, content], |row| row.get(0)).map_err(crate::docbase::sql_err)?;
                        rows.collect::<Result<_, _>>().map_err(crate::docbase::sql_err)?
                    };
                    if mentions.is_empty() {
                        return Err("Esa ficha no aparece en ese documento".to_string());
                    }
                    let decisions: Vec<IdentityDecision> = mentions
                        .iter()
                        .map(|m| IdentityDecision::Separate { a: format!("mention:{m}"), b: format!("entity:{entity}") })
                        .collect();
                    base.knowledge_decide_many(&decisions, Some(note))?
                }
                (operation, _) => {
                    if entities.len() != 2 {
                        return Err("Hacen falta las dos fichas".to_string());
                    }
                    let (a, b) = (format!("entity:{}", entities[0]), format!("entity:{}", entities[1]));
                    let decision = if operation == "fichas_unir" { IdentityDecision::Join { a, b } } else { IdentityDecision::Separate { a, b } };
                    vec![base.knowledge_decide(&decision, Some(note), Some(&shown))?]
                }
            };
            Ok(json!({ "decisions": ids }))
        }
        "documento_releer" => {
            if crate::docbase::similar::classify::ask_engine(&base.conn())?.is_none() {
                return Err("Falta elegir el motor de «Preguntar al modelo» en Ajustes → Servicios de IA: sin él no se puede volver a leer.".to_string());
            }
            let mut requested = 0;
            for item in card.items.iter().filter(|i| i.key.as_ref().is_some_and(|k| keys.contains(k))) {
                let Some(location_id) = item.location_id else { continue };
                requested += base.knowledge_request_reread(&crate::docbase::knowledge::extract::RereadTarget { location_id: Some(location_id), ..Default::default() })?;
            }
            Ok(json!({ "requested": requested }))
        }
        "consulta_guardar" => {
            let saved = base.save_query(&text("titulo"), card.request.as_deref().unwrap_or_default(), &text("respuesta"), &[])?;
            Ok(json!({ "query": saved.id }))
        }
        other => Err(format!("«{other}» todavía no se puede aplicar")),
    }
}

fn snapshot(undo: &Value) -> Result<MatterSnapshot, String> {
    serde_json::from_value(undo.get("snapshot").cloned().unwrap_or(Value::Null)).map_err(|e| format!("No se puede deshacer: falta cómo estaba ({e})"))
}

fn decisions(undo: &Value) -> Vec<i64> {
    undo.get("decisions").and_then(Value::as_array).map(|a| a.iter().filter_map(Value::as_i64).collect()).unwrap_or_default()
}

/// Undoes what `apply` did, with what it returned.
pub fn undo(base: &DocBase, card: &Card, undo: &Value) -> Result<(), String> {
    match card.operation.as_str() {
        "caso_agregar_documentos" | "caso_sacar_documentos" => {
            for id in decisions(undo) {
                // A decision undone by hand meanwhile is already undone.
                if let Err(error) = base.knowledge_undo(id) {
                    if !error.contains("ya se deshizo") {
                        return Err(error);
                    }
                }
            }
            base.knowledge_restore_matter(&snapshot(undo)?, false)
        }
        "caso_crear" => {
            let id = undo.get("created").and_then(Value::as_str).ok_or_else(|| "No se puede deshacer: falta el caso creado".to_string())?;
            base.knowledge_decide_matter(id, "discard")
        }
        "caso_renombrar" => {
            let before = snapshot(undo)?;
            if before.name_by_person {
                base.knowledge_rename_matter(&before.id, &before.name)
            } else {
                base.knowledge_rename_matter(&before.id, "")
            }
        }
        "caso_decidir" => base.knowledge_restore_matter(&snapshot(undo)?, true),
        "caso_cambiar_estado" => {
            let matter = undo.get("matter").and_then(Value::as_str).ok_or_else(|| "No se puede deshacer: falta el caso".to_string())?;
            let status = undo.get("status").and_then(Value::as_str).unwrap_or("open");
            base.knowledge_set_matter_status(matter, status)
        }
        "fichas_unir" | "fichas_separar" => {
            for id in decisions(undo) {
                if let Err(error) = base.knowledge_undo(id) {
                    if !error.contains("ya se deshizo") {
                        return Err(error);
                    }
                }
            }
            Ok(())
        }
        "consulta_guardar" => {
            let id = undo.get("query").and_then(Value::as_i64).ok_or_else(|| "No se puede deshacer: falta la consulta".to_string())?;
            base.delete_saved_query(id)
        }
        other => Err(format!("«{other}» no se deshace")),
    }
}

/// What a card says once it is done.
pub fn done_text(card: &Card, keys: &[String]) -> String {
    let words = super::words::words();
    let n = contents(keys).len();
    let documentos = if n == 1 { "1 documento".to_string() } else { format!("{n} documentos") };
    let name = card.target.as_ref().and_then(|t| t.name.clone()).unwrap_or_default();
    match card.operation.as_str() {
        "caso_agregar_documentos" => format!("Hecho: {documentos} agregados al {} «{name}».", words.caso),
        "caso_sacar_documentos" => format!("Hecho: {documentos} sacados del {} «{name}».", words.caso),
        "caso_crear" => format!("Hecho: se creó el {} «{name}».", words.caso),
        "caso_renombrar" => format!("Hecho: el {} se llama ahora «{}».", words.caso, card.input.get("nombre").and_then(Value::as_str).unwrap_or_default()),
        "caso_decidir" => {
            let word = match card.input.get("decision").and_then(Value::as_str).unwrap_or_default() {
                "confirmar" => "confirmado",
                "descartar" => "descartado",
                _ => "propuesto otra vez",
            };
            format!("Hecho: el {} «{name}» quedó {word}.", words.caso)
        }
        "caso_cambiar_estado" => {
            let word = card.input.get("estado").and_then(Value::as_str).unwrap_or("abierto");
            format!("Hecho: el {} «{name}» quedó {word}.", words.caso)
        }
        "fichas_unir" => format!("Hecho: las dos {} son ahora una.", words.fichas),
        "fichas_separar" => "Hecho: quedaron separadas.".to_string(),
        "consulta_guardar" => "Hecho: la consulta quedó guardada.".to_string(),
        "documento_releer" => format!("Hecho: {documentos} esperan su lectura nueva; la app los lee en segundo plano."),
        _ => "Hecho.".to_string(),
    }
}
