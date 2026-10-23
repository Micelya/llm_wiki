//! The operations of the contract that only read (2.3). Each result names
//! every object by the identifier the model uses to ask for it again
//! (`doc-<content>`, `ent-…`, `mat-…`) and, for a document, its name and
//! where it is. The keys are in Spanish, as the data the model writes.
//! `ruta`, when there is one, is the page of the object: what the chat
//! cites and opens.

use std::collections::BTreeSet;

use rusqlite::OptionalExtension;
use serde_json::{json, Value};

use crate::docbase::knowledge::articles::kind_of;
use crate::docbase::{sql_err, DocBase, Visible};

use super::resolve::{self, Found};
use super::{find, Kind};

/// About how much of a long text one part has (4.3).
pub const PART_CHARS: usize = 12_000;
/// The most results of a search.
const MAX_RESULTS: usize = 10;
const DEFAULT_RESULTS: usize = 5;

/// What a reading needs besides the base: the documents attached to the
/// message (by the id of their location), for «este documento».
#[derive(Debug, Clone, Default)]
pub struct ReadContext {
    pub attached: Vec<i64>,
}

/// Runs an operation that only reads, with its data already checked
/// (`Operation::check`).
pub fn read(base: &DocBase, name: &str, data: &Value, context: &ReadContext) -> Result<Value, String> {
    let operation = find(name).ok_or_else(|| format!("«{name}» no es una operación del contrato"))?;
    if operation.kind != Kind::Read {
        return Err(format!("«{name}» cambia algo: no se lee, se propone"));
    }
    let text = |key: &str| data.get(key).and_then(Value::as_str).unwrap_or_default().to_string();
    let number = |key: &str| data.get(key).and_then(Value::as_i64);
    let list = |key: &str| -> Vec<String> {
        data.get(key).and_then(Value::as_array).map(|items| items.iter().filter_map(|v| v.as_str().map(str::to_string)).collect()).unwrap_or_default()
    };
    match name {
        "buscar" => search(base, &text("consulta"), count(number("cantidad")), data.get("tipo").and_then(Value::as_str)),
        "buscar_pasajes" => passages(base, &text("consulta"), &list("documentos"), data.get("caso").and_then(Value::as_str), count(number("cantidad")), context),
        "leer" => read_article(base, &text("id"), number("parte"), context),
        "leer_texto" => read_text(base, &text("documento"), number("parte"), context),
        "relaciones" => relations(base, &text("nombre")),
        "ver_documento" => see_document(base, &text("documento"), context),
        "ver_ficha" => see_entity(base, &text("ficha")),
        "ver_caso" => see_matter(base, &text("caso")),
        "listar_casos" => list_matters(base, data.get("incluir_cerrados").and_then(Value::as_bool).unwrap_or(false)),
        "listar_plantillas" => list_templates(base),
        "estado_pedido" => request_state(base, number("pedido").unwrap_or_default()),
        other => Err(format!("«{other}» todavía no se puede leer")),
    }
}

fn count(asked: Option<i64>) -> usize {
    asked.map(|n| n.clamp(1, MAX_RESULTS as i64) as usize).unwrap_or(DEFAULT_RESULTS)
}

/// The words of a matter's state.
pub fn status_word(status: &str) -> &'static str {
    match status {
        "closed" => "cerrado",
        "archived" => "archivado",
        _ => "abierto",
    }
}

fn confirmation_word(confirmation: &str) -> &'static str {
    match confirmation {
        "confirmed" => "confirmado",
        "discarded" => "descartado",
        _ => "propuesto",
    }
}

/// A part of a long text: the `part`-th (from 1) of about `PART_CHARS`
/// characters, cut at a line break when there is one near, and how many
/// parts there are.
pub fn part_of(text: &str, part: Option<i64>) -> (String, usize, usize) {
    let chars: Vec<char> = text.chars().collect();
    if chars.len() <= PART_CHARS {
        return (text.to_string(), 1, 1);
    }
    let mut cuts = vec![0usize];
    let mut at = 0usize;
    while chars.len() - at > PART_CHARS {
        let mut end = at + PART_CHARS;
        // Back to a line break in the last fifth, if there is one.
        let floor = at + PART_CHARS * 4 / 5;
        if let Some(newline) = (floor..end).rev().find(|&i| chars[i] == '\n') {
            end = newline + 1;
        }
        cuts.push(end);
        at = end;
    }
    cuts.push(chars.len());
    let total = cuts.len() - 1;
    let wanted = part.unwrap_or(1).clamp(1, total as i64) as usize;
    let piece: String = chars[cuts[wanted - 1]..cuts[wanted]].iter().collect();
    (piece, wanted, total)
}

fn page_of_document(base: &DocBase, content: i64) -> Result<Option<String>, String> {
    Ok(base.document_pages()?.remove(&content))
}

fn search(base: &DocBase, query: &str, limit: usize, only: Option<&str>) -> Result<Value, String> {
    let query = query.trim();
    if query.is_empty() {
        return Err("Falta qué buscar".to_string());
    }
    let mut out: Vec<Value> = Vec::new();
    let mut seen: BTreeSet<String> = BTreeSet::new();
    let pages = base.document_pages()?;
    let wants = |kind: &str| only.is_none_or(|o| o == kind);
    // The pages of the base the search finds.
    if let Ok(found) = crate::commands::search::search_base(base, query, limit) {
        for result in found.results {
            let Some(object) = base.knowledge_object_of_page(&result.path)? else { continue };
            let tipo = match object.kind.as_str() {
                "document" => "documento",
                "entity" => "ficha",
                "matter" => "caso",
                _ => continue,
            };
            let plural = match tipo {
                "documento" => "documentos",
                "ficha" => "fichas",
                _ => "casos",
            };
            if !wants(plural) || !seen.insert(object.object_id.clone()) {
                continue;
            }
            let mut item = json!({ "id": object.object_id, "tipo": tipo, "titulo": result.title, "pasaje": result.snippet, "ruta": result.path });
            if tipo == "documento" {
                if let Some(content) = object.content_id {
                    if let Some(line) = resolve::document_line(&base.conn(), content)? {
                        item["id"] = json!(line.key);
                        item["nombre"] = json!(line.name);
                        item["lugar"] = json!(line.place);
                        seen.insert(line.key);
                    }
                }
            }
            if tipo == "caso" {
                if let Some(line) = resolve::matter_line(&base.conn(), &object.object_id)? {
                    item["estado"] = json!(status_word(&line.status));
                }
            }
            out.push(item);
        }
    }
    // The documents whose text says it, when the pages did not bring enough.
    if wants("documentos") && out.len() < limit {
        let found = base.search_documents(&Visible::All, query, limit, None)?;
        for hit in found.hits {
            let key = format!("doc-{}", hit.content_id);
            if !seen.insert(key.clone()) {
                continue;
            }
            let Some(line) = resolve::document_line(&base.conn(), hit.content_id)? else { continue };
            out.push(json!({
                "id": key, "tipo": "documento", "titulo": line.name, "nombre": line.name, "lugar": line.place,
                "pasaje": hit.snippet, "ruta": pages.get(&hit.content_id),
            }));
            if out.len() >= limit {
                break;
            }
        }
    }
    // The matters and the records by their name.
    if wants("casos") {
        let matters = match resolve::find_matter(base, query)? {
            Found::One(line) => vec![line],
            Found::Many(lines) => lines,
            Found::None => Vec::new(),
        };
        for line in matters {
            if seen.insert(line.key.clone()) {
                out.push(json!({
                    "id": line.key, "tipo": "caso", "titulo": line.name, "estado": status_word(&line.status),
                    "confirmacion": confirmation_word(&line.confirmation), "documentos": line.documents,
                }));
            }
        }
    }
    if wants("fichas") {
        let entities = match resolve::find_entity(base, query, None)? {
            Found::One(line) => vec![line],
            Found::Many(lines) => lines,
            Found::None => Vec::new(),
        };
        for line in entities {
            if seen.insert(line.key.clone()) {
                out.push(json!({ "id": line.key, "tipo": "ficha", "titulo": line.name, "clase": line.kind }));
            }
        }
    }
    out.truncate(limit.max(1) * 2);
    Ok(json!({ "consulta": query, "resultados": out }))
}

fn contents_of_matter(base: &DocBase, matter: &str) -> Result<Option<Vec<i64>>, String> {
    let line = match resolve::find_matter(base, matter)? {
        Found::One(line) => line,
        _ => return Ok(None),
    };
    let conn = base.conn();
    let mut statement = conn.prepare("SELECT content_id FROM knowledge_matter_document WHERE matter_id = ?1").map_err(sql_err)?;
    let rows = statement.query_map([&line.key], |row| row.get(0)).map_err(sql_err)?;
    Ok(Some(rows.collect::<Result<_, _>>().map_err(sql_err)?))
}

fn passages(base: &DocBase, query: &str, documents: &[String], matter: Option<&str>, limit: usize, context: &ReadContext) -> Result<Value, String> {
    let query = query.trim();
    if query.is_empty() {
        return Err("Falta qué buscar".to_string());
    }
    let mut within: Option<Vec<i64>> = None;
    if !documents.is_empty() {
        let mut contents = Vec::new();
        for named in documents {
            if let Found::One(line) = resolve::find_document(base, named, &context.attached)? {
                contents.push(line.content_id);
            }
        }
        within = Some(contents);
    }
    if let Some(matter) = matter.filter(|m| !m.trim().is_empty()) {
        let Some(contents) = contents_of_matter(base, matter)? else {
            return Err(format!("No encontré el caso «{matter}»"));
        };
        within = Some(match within {
            Some(already) => already.into_iter().filter(|c| contents.contains(c)).collect(),
            None => contents,
        });
    }
    if within.as_ref().is_some_and(|w| w.is_empty()) {
        return Ok(json!({ "consulta": query, "pasajes": [] }));
    }
    let found = base.search_documents(&Visible::All, query, limit, within.as_deref())?;
    let pages = base.document_pages()?;
    let mut out = Vec::new();
    for hit in found.hits {
        let Some(line) = resolve::document_line(&base.conn(), hit.content_id)? else { continue };
        out.push(json!({
            "id": line.key, "nombre": line.name, "lugar": line.place, "pasaje": hit.snippet, "ruta": pages.get(&hit.content_id),
        }));
    }
    Ok(json!({ "consulta": query, "pasajes": out }))
}

fn candidates_of<T: serde::Serialize>(lines: &[T]) -> Value {
    json!(lines)
}

/// What a name or an id names, for `leer`: one object, several, or none.
enum Named {
    Object(String),
    Many(Value),
    None,
}

fn named_object(base: &DocBase, named: &str, context: &ReadContext) -> Result<Named, String> {
    let named = named.trim();
    if kind_of(named).is_some() && !named.contains(' ') {
        return Ok(Named::Object(named.to_string()));
    }
    if let Found::One(line) = resolve::find_document(base, named, &context.attached)? {
        if resolve::names_the_attached(named) || named.starts_with("loc-") || named.starts_with("location:") || named.starts_with("content:") {
            return Ok(Named::Object(line.key));
        }
    }
    let mut many: Vec<Value> = Vec::new();
    match resolve::find_matter(base, named)? {
        Found::One(line) => return Ok(Named::Object(line.key)),
        Found::Many(lines) => many.extend(lines.iter().map(|l| json!({ "id": l.key, "tipo": "caso", "nombre": l.name }))),
        Found::None => {}
    }
    match resolve::find_entity(base, named, None)? {
        Found::One(line) if many.is_empty() => return Ok(Named::Object(line.key)),
        Found::One(line) => many.push(json!({ "id": line.key, "tipo": "ficha", "nombre": line.name })),
        Found::Many(lines) => many.extend(lines.iter().map(|l| json!({ "id": l.key, "tipo": "ficha", "nombre": l.name }))),
        Found::None => {}
    }
    match resolve::find_document(base, named, &context.attached)? {
        Found::One(line) if many.is_empty() => return Ok(Named::Object(line.key)),
        Found::One(line) => many.push(json!({ "id": line.key, "tipo": "documento", "nombre": line.name, "lugar": line.place })),
        Found::Many(lines) => many.extend(lines.iter().map(|l| json!({ "id": l.key, "tipo": "documento", "nombre": l.name, "lugar": l.place }))),
        Found::None => {}
    }
    Ok(if many.is_empty() { Named::None } else { Named::Many(json!(many)) })
}

fn read_article(base: &DocBase, named: &str, part: Option<i64>, context: &ReadContext) -> Result<Value, String> {
    let id = match named_object(base, named, context)? {
        Named::Object(id) => id,
        Named::Many(candidates) => return Ok(json!({ "varios": candidates, "aviso": "Ese nombre coincide con varios: leé uno por su identificador." })),
        Named::None => return Err(format!("No encontré «{named}» en la base")),
    };
    match base.knowledge_article(&id)? {
        Some(article) => {
            let (text, at, total) = part_of(&article.body, part);
            Ok(json!({ "id": id, "titulo": article.title, "ruta": article.path, "parte": at, "partes": total, "texto": text }))
        }
        // A document without an article yet: its own text.
        None if id.starts_with("doc-") => read_text(base, &id, part, context),
        None => Err(format!("«{id}» no tiene artículo todavía")),
    }
}

fn document_of(base: &DocBase, named: &str, context: &ReadContext) -> Result<resolve::DocLine, String> {
    match resolve::find_document(base, named, &context.attached)? {
        Found::One(line) => Ok(line),
        Found::Many(lines) => Err(format!(
            "«{named}» coincide con varios documentos: {}",
            lines.iter().map(|l| format!("{} ({})", l.name, l.key)).collect::<Vec<_>>().join("; ")
        )),
        Found::None => Err(format!("No encontré el documento «{named}»")),
    }
}

fn read_text(base: &DocBase, named: &str, part: Option<i64>, context: &ReadContext) -> Result<Value, String> {
    let line = document_of(base, named, context)?;
    let body: Option<String> = base
        .conn()
        .query_row("SELECT body FROM derivative WHERE content_id = ?1 AND kind = 'text'", [line.content_id], |row| row.get(0))
        .optional()
        .map_err(sql_err)?;
    let text: String = body
        .unwrap_or_default()
        .lines()
        .filter(|l| !l.trim_start().starts_with("<!-- texto-reconocido"))
        .collect::<Vec<_>>()
        .join("\n");
    if text.trim().is_empty() {
        return Err(format!("«{}» no tiene texto todavía", line.name));
    }
    let (piece, at, total) = part_of(&text, part);
    Ok(json!({
        "id": line.key, "nombre": line.name, "lugar": line.place, "parte": at, "partes": total, "texto": piece,
        "ruta": page_of_document(base, line.content_id)?,
    }))
}

fn relations(base: &DocBase, name: &str) -> Result<Value, String> {
    let found = crate::agent::base_tools::base_graph_search(base, name, MAX_RESULTS)?;
    let items: Vec<Value> = found
        .into_iter()
        .map(|r| json!({ "titulo": r.title, "ruta": r.path, "relaciones": r.snippet }))
        .collect();
    Ok(json!({ "nombre": name, "resultados": items }))
}

fn see_document(base: &DocBase, named: &str, context: &ReadContext) -> Result<Value, String> {
    let line = document_of(base, named, context)?;
    let mut out = json!({ "id": line.key, "nombre": line.name, "lugar": line.place, "fecha": line.date, "ruta": page_of_document(base, line.content_id)? });
    if let Some(location) = line.location_id {
        if let Some(detail) = base.document(&Visible::All, location)? {
            out["tipo"] = json!(detail.row.doc_type_name.clone().or(detail.row.doc_type.clone()));
            out["copias"] = json!(detail.locations.len());
            out["etiquetas"] = json!(detail.row.labels.iter().map(|l| l.name.clone()).collect::<Vec<_>>());
        }
        let known = base.knowledge_document_entities(&Visible::All, location)?;
        out["personas_y_organizaciones"] = json!(known
            .entities
            .iter()
            .map(|e| json!({ "id": e.entity_id, "nombre": e.name, "clase": e.kind, "papeles": e.roles }))
            .collect::<Vec<_>>());
        out["casos"] = json!(known
            .matters
            .iter()
            .map(|m| json!({ "id": m.matter_id, "nombre": m.name, "propuesto": m.proposed }))
            .collect::<Vec<_>>());
    }
    Ok(out)
}

fn see_entity(base: &DocBase, named: &str) -> Result<Value, String> {
    let line = match resolve::find_entity(base, named, None)? {
        Found::One(line) => line,
        Found::Many(lines) => return Ok(json!({ "varios": candidates_of(&lines), "aviso": "Ese nombre coincide con varias: pedí una por su identificador." })),
        Found::None => return Err(format!("No encontré a «{named}»")),
    };
    let Some(detail) = base.knowledge_entity(&Visible::All, &line.key)? else {
        return Err(format!("No encontré a «{named}»"));
    };
    let documents: BTreeSet<i64> = detail.mentions.iter().map(|m| m.content_id).collect();
    Ok(json!({
        "id": detail.id, "nombre": detail.name, "clase": detail.kind, "ruta": detail.page_path,
        "se_escribe": detail.aliases.iter().map(|a| a.written.clone()).collect::<Vec<_>>(),
        "identificadores": detail.identifiers.iter().map(|i| json!({ "clase": i.kind, "valor": i.value })).collect::<Vec<_>>(),
        "documentos": documents.iter().take(20).map(|c| format!("doc-{c}")).collect::<Vec<_>>(),
        "cantidad_de_documentos": documents.len(),
        "casos": detail.matters.iter().map(|m| json!({ "id": m.matter_id, "nombre": m.name })).collect::<Vec<_>>(),
        "decisiones": detail.decisions.iter().filter(|d| d.state == "active").count(),
    }))
}

fn see_matter(base: &DocBase, named: &str) -> Result<Value, String> {
    let line = match resolve::find_matter(base, named)? {
        Found::One(line) => line,
        Found::Many(lines) => return Ok(json!({ "varios": candidates_of(&lines), "aviso": "Ese nombre coincide con varios: pedí uno por su identificador." })),
        Found::None => return Err(format!("No encontré el caso «{named}»")),
    };
    let Some(detail) = base.knowledge_matter(&Visible::All, &line.key)? else {
        return Err(format!("No encontré el caso «{named}»"));
    };
    let mut documents = Vec::new();
    for d in &detail.documents {
        let place = resolve::document_line(&base.conn(), d.document.content_id)?.map(|l| l.place).unwrap_or_default();
        documents.push(json!({
            "id": format!("doc-{}", d.document.content_id), "nombre": d.document.name, "lugar": place,
            "estado": if d.state == "proposed" { "propuesto" } else { "firme" },
        }));
    }
    Ok(json!({
        "id": detail.id, "nombre": detail.name, "estado": status_word(&detail.status), "confirmacion": confirmation_word(&detail.confirmation),
        "creado_a_mano": detail.origin == "person", "ruta": detail.page_path, "documentos": documents,
        "partes": detail.parties.iter().map(|p| json!({ "id": p.entity_id, "nombre": p.name, "papeles": p.roles })).collect::<Vec<_>>(),
        "decisiones": detail.decisions.iter().filter(|d| d.state == "active").count(),
    }))
}

fn list_matters(base: &DocBase, closed_too: bool) -> Result<Value, String> {
    let matters = base.knowledge_matters(&Visible::All)?;
    let items: Vec<Value> = matters
        .into_iter()
        .filter(|m| closed_too || m.status == "open")
        .map(|m| {
            json!({
                "id": m.id, "nombre": m.name, "estado": status_word(&m.status), "confirmacion": confirmation_word(&m.confirmation),
                "documentos": m.documents, "propuestos": m.proposed, "ruta": m.page_path,
            })
        })
        .collect();
    Ok(json!({ "casos": items }))
}

fn list_templates(base: &DocBase) -> Result<Value, String> {
    let templates = base.templates(&Visible::All)?;
    let items: Vec<Value> = templates
        .into_iter()
        .map(|t| {
            json!({
                "nombre": t.name.clone().unwrap_or_else(|| t.principal.name.clone()),
                "documento_principal": format!("doc-{}", t.principal.content_id),
                "tipo": t.kind.proposed_type_name.clone().or(t.kind.doc_type.clone()),
                "estado": if t.state == "confirmed" { "confirmada" } else { "propuesta" },
                "documentos": t.documents, "campos": t.fields,
            })
        })
        .collect();
    Ok(json!({ "plantillas": items }))
}

fn request_state(base: &DocBase, id: i64) -> Result<Value, String> {
    let Some(entry) = super::log::get(base, id)? else { return Err(format!("No hay un pedido número {id}")) };
    let state = match entry.state.as_str() {
        "proposed" => "pendiente",
        "applied" => "hecho",
        "cancelled" => "cancelado",
        "undone" => "deshecho",
        "expired" => "vencido",
        _ => "falló",
    };
    Ok(json!({ "pedido": id, "operacion": entry.operation, "estado": state }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_long_text_is_read_in_parts_that_cover_it_all() {
        let line = "una línea del texto de prueba\n";
        let text = line.repeat(1_200);
        let (first, at, total) = part_of(&text, None);
        assert_eq!(at, 1);
        assert!(total >= 3);
        assert!(first.chars().count() <= PART_CHARS);
        let mut whole = String::new();
        for n in 1..=total {
            whole.push_str(&part_of(&text, Some(n as i64)).0);
        }
        assert_eq!(whole, text);
        assert_eq!(part_of("corto", Some(3)), ("corto".to_string(), 1, 1));
    }
}
