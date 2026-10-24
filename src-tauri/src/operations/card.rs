//! The card a person confirms (4.2 and 11.3): what the operation will do,
//! with the verb and the count; on what, each thing as the base keeps it,
//! with a way to open it; the request that originated it; what the screen
//! already warns of; who proposed it. Everything on it comes from the
//! base, not from what the model wrote. Its fingerprint is the state of
//! what it will change: when it differs at the moment of confirming, the
//! card is shown again.

use std::collections::BTreeSet;

use rusqlite::{Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use crate::docbase::{sql_err, DocBase, Visible};

use super::read::status_word;
use super::resolve::{self, DocLine, EntityLine, Found, MatterLine};
use super::words::capitalized;
use super::{find, Operation, Undo, MAX_CARD_ITEMS, SECOND_CONFIRMATION_OVER};

/// One thing of the base a card can show for a person to choose.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Candidate {
    pub key: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub place: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub location_id: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub date: Option<String>,
}

/// What the card does on: a matter, a new matter, nothing.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Target {
    /// `caso` or `nuevo_caso`.
    pub kind: String,
    /// What the model named.
    pub named: String,
    /// The matter's id (none for a new one or while it is to be chosen).
    #[serde(default)]
    pub key: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    /// `ok`, `not_found` or `choose`.
    pub status: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub candidates: Vec<Candidate>,
    /// `proposed`, `confirmed` or `discarded`.
    #[serde(default)]
    pub confirmation: Option<String>,
    /// `abierto`, `cerrado` or `archivado`.
    #[serde(default)]
    pub status_word: Option<String>,
}

/// One thing the card touches.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CardItem {
    /// What the model named.
    pub named: String,
    /// What it is in the base (`doc-41`, `ent-…`), once found.
    #[serde(default)]
    pub key: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub place: Option<String>,
    #[serde(default)]
    pub location_id: Option<i64>,
    #[serde(default)]
    pub date: Option<String>,
    /// `ok`, `unchanged` (already as asked), `not_found`, `choose` (with
    /// candidates) or `blocked` (it cannot be: with the reason).
    pub status: String,
    pub checked: bool,
    /// Both sides of a decision about records: they cannot be unticked.
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub reason: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub candidates: Vec<Candidate>,
    /// For a record: what the screen shows when deciding (its name, how it
    /// is written, its identifiers, its documents).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<Value>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProposedBy {
    /// `chat`, `screen` or `program:<id>`.
    pub caller: String,
    #[serde(default)]
    pub engine: Option<String>,
}

/// What a card costs before it is confirmed (`documento_releer`): how many
/// documents and how many questions to the model.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Cost {
    pub documents: usize,
    pub questions: usize,
}

/// What «Abrir» opens after it is done.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Open {
    /// `caso` or `ficha`.
    pub kind: String,
    pub id: String,
}

/// A card, as it travels to the screen.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Card {
    /// Its number in the record.
    pub id: i64,
    pub operation: String,
    pub title: String,
    /// The request of the person that originated it.
    #[serde(default)]
    pub request: Option<String>,
    #[serde(default)]
    pub target: Option<Target>,
    pub items: Vec<CardItem>,
    pub warnings: Vec<String>,
    /// Shown with what will be done (the new name, the answer to keep).
    #[serde(default)]
    pub note: Option<String>,
    /// What it costs, said before (questions to the model).
    #[serde(default)]
    pub cost: Option<Cost>,
    /// With more than ten things ticked, or a cost, «Confirmar» asks again.
    pub second_confirmation: bool,
    pub proposed_by: ProposedBy,
    /// `proposed`, `applied`, `cancelled`, `undone`, `failed`.
    pub state: String,
    pub fingerprint: String,
    /// Why it cannot be confirmed as it is, if so.
    #[serde(default)]
    pub blocked: Option<String>,
    /// After it is done or undone: what happened.
    #[serde(default)]
    pub result: Option<String>,
    pub undoable: bool,
    /// Why it cannot be undone, said before.
    #[serde(default)]
    pub undo_note: Option<String>,
    #[serde(default)]
    pub open: Option<Open>,
    /// The data it was proposed with, checked.
    pub input: Value,
}

fn digest(text: &str) -> String {
    Sha256::digest(text.as_bytes()).iter().take(12).map(|b| format!("{b:02x}")).collect()
}

fn list(input: &Value, key: &str) -> Vec<String> {
    input.get(key).and_then(Value::as_array).map(|a| a.iter().filter_map(|v| v.as_str().map(str::to_string)).collect()).unwrap_or_default()
}

fn text(input: &Value, key: &str) -> String {
    input.get(key).and_then(Value::as_str).unwrap_or_default().trim().to_string()
}

fn candidate_of_document(line: &DocLine) -> Candidate {
    Candidate { key: line.key.clone(), name: line.name.clone(), place: Some(line.place.clone()), location_id: line.location_id, date: line.date.clone() }
}

fn candidate_of_entity(line: &EntityLine, conn: &Connection) -> Result<Candidate, String> {
    let identifiers = identifiers_of(conn, &line.key)?;
    Ok(Candidate {
        key: line.key.clone(),
        name: line.name.clone(),
        place: (!identifiers.is_empty()).then(|| identifiers.join(", ")),
        location_id: None,
        date: None,
    })
}

fn identifiers_of(conn: &Connection, entity: &str) -> Result<Vec<String>, String> {
    let mut statement = conn
        .prepare(
            "SELECT DISTINCT i.kind, i.value FROM knowledge_identifier i JOIN knowledge_mention m ON m.id = i.mention_id
             WHERE m.entity_id = ?1 ORDER BY i.kind, i.value",
        )
        .map_err(sql_err)?;
    let rows = statement.query_map([entity], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))).map_err(sql_err)?;
    let vocabulary = crate::docbase::knowledge::vocabulary::vocabulary();
    let mut out = Vec::new();
    for row in rows {
        let (kind, value) = row.map_err(sql_err)?;
        if vocabulary.is_strong(&kind) {
            let label = vocabulary.identifiers.iter().find(|t| t.id == kind).map(|t| t.label.clone()).unwrap_or(kind);
            out.push(format!("{label} {value}"));
        }
    }
    Ok(out)
}

fn matter_candidate(line: &MatterLine) -> Candidate {
    Candidate { key: line.key.clone(), name: line.name.clone(), place: Some(format!("{} documentos", line.documents)), location_id: None, date: None }
}

/// The item of a document the model named, resolved in the base.
fn document_item(base: &DocBase, named: &str, attached: &[i64]) -> Result<Vec<CardItem>, String> {
    let blank = |status: &str| CardItem {
        named: named.to_string(),
        key: None,
        name: None,
        place: None,
        location_id: None,
        date: None,
        status: status.to_string(),
        checked: false,
        required: false,
        reason: None,
        candidates: Vec::new(),
        detail: None,
    };
    Ok(match resolve::find_document(base, named, attached)? {
        Found::One(line) => vec![with_document(blank("ok"), &line)],
        // «Estos» are every attached document, each one an item.
        Found::Many(lines) if resolve::names_the_attached(named) => lines.iter().map(|line| with_document(blank("ok"), line)).collect(),
        Found::Many(lines) => {
            let mut item = blank("choose");
            item.candidates = lines.iter().map(candidate_of_document).collect();
            vec![item]
        }
        Found::None => vec![blank("not_found")],
    })
}

fn with_document(mut item: CardItem, line: &DocLine) -> CardItem {
    item.key = Some(line.key.clone());
    item.name = Some(line.name.clone());
    item.place = Some(line.place.clone());
    item.location_id = line.location_id;
    item.date = line.date.clone();
    item.checked = true;
    item
}

fn entity_item(base: &DocBase, named: &str, kind: Option<&str>) -> Result<CardItem, String> {
    let mut item = CardItem {
        named: named.to_string(),
        key: None,
        name: None,
        place: None,
        location_id: None,
        date: None,
        status: "not_found".into(),
        checked: false,
        required: true,
        reason: None,
        candidates: Vec::new(),
        detail: None,
    };
    match resolve::find_entity(base, named, kind)? {
        Found::One(line) => {
            item.key = Some(line.key.clone());
            item.name = Some(line.name.clone());
            item.status = "ok".into();
            item.checked = true;
        }
        Found::Many(lines) => {
            let conn = base.conn();
            item.status = "choose".into();
            item.candidates = lines.iter().map(|l| candidate_of_entity(l, &conn)).collect::<Result<_, _>>()?;
        }
        Found::None => {}
    }
    Ok(item)
}

fn matter_target(base: &DocBase, named: &str) -> Result<Target, String> {
    let mut target = Target {
        kind: "caso".into(),
        named: named.to_string(),
        key: None,
        name: None,
        status: "not_found".into(),
        candidates: Vec::new(),
        confirmation: None,
        status_word: None,
    };
    match resolve::find_matter(base, named)? {
        Found::One(line) => {
            target.key = Some(line.key);
            target.status = "ok".into();
        }
        Found::Many(lines) => {
            target.status = "choose".into();
            target.candidates = lines.iter().map(matter_candidate).collect();
        }
        Found::None => {}
    }
    Ok(target)
}

/// Builds the card of a proposal: what the model named, found in the
/// base, and how it all is now. The data are already checked.
pub fn build(base: &DocBase, operation: &Operation, input: &Value, request: Option<&str>, proposed_by: ProposedBy, attached: &[i64]) -> Result<Card, String> {
    let mut target = None;
    let mut items = Vec::new();
    match operation.name {
        "caso_agregar_documentos" | "caso_sacar_documentos" => {
            target = Some(matter_target(base, &text(input, "caso"))?);
            for named in list(input, "documentos") {
                for item in document_item(base, &named, attached)? {
                    if !items.iter().any(|i: &CardItem| i.key.is_some() && i.key == item.key) {
                        items.push(item);
                    }
                }
            }
        }
        "caso_crear" => {
            target = Some(Target {
                kind: "nuevo_caso".into(),
                named: text(input, "nombre"),
                key: None,
                name: Some(text(input, "nombre").split_whitespace().collect::<Vec<_>>().join(" ")),
                status: "ok".into(),
                candidates: Vec::new(),
                confirmation: None,
                status_word: None,
            });
            for named in list(input, "documentos") {
                for item in document_item(base, &named, attached)? {
                    if !items.iter().any(|i: &CardItem| i.key.is_some() && i.key == item.key) {
                        items.push(item);
                    }
                }
            }
        }
        "caso_renombrar" | "caso_decidir" | "caso_cambiar_estado" => {
            target = Some(matter_target(base, &text(input, "caso"))?);
        }
        "fichas_unir" => {
            for named in list(input, "fichas").iter().take(2) {
                items.push(entity_item(base, named, None)?);
            }
        }
        "fichas_separar" => {
            let fichas = list(input, "fichas");
            if fichas.len() >= 2 {
                for named in fichas.iter().take(2) {
                    items.push(entity_item(base, named, None)?);
                }
            } else {
                let ficha = fichas.first().cloned().unwrap_or_else(|| text(input, "ficha"));
                items.push(entity_item(base, &ficha, None)?);
                let document = text(input, "documento");
                for mut item in document_item(base, &document, attached)? {
                    item.required = true;
                    items.push(item);
                }
            }
        }
        "documento_releer" => {
            for named in list(input, "documentos") {
                for item in document_item(base, &named, attached)? {
                    if !items.iter().any(|i: &CardItem| i.key.is_some() && i.key == item.key) {
                        items.push(item);
                    }
                }
            }
        }
        "consulta_guardar" => {}
        other => return Err(format!("«{other}» todavía no se puede proponer desde el chat")),
    }
    if items.len() > MAX_CARD_ITEMS {
        return Err(format!(
            "Son {} elementos: más de {MAX_CARD_ITEMS} no entran en una tarjeta. Hacelo por partes o desde la pantalla de Casos.",
            items.len()
        ));
    }
    let mut card = Card {
        id: 0,
        operation: operation.name.to_string(),
        title: String::new(),
        request: request.map(str::to_string),
        target,
        items,
        warnings: Vec::new(),
        note: None,
        cost: None,
        second_confirmation: false,
        proposed_by,
        state: "proposed".into(),
        fingerprint: String::new(),
        blocked: None,
        result: None,
        undoable: !matches!(operation.undo, Undo::Not(_)),
        undo_note: match operation.undo {
            Undo::Not(why) => Some(why.to_string()),
            _ => None,
        },
        open: None,
        input: input.clone(),
    };
    refresh(base, &mut card, true)?;
    Ok(card)
}

/// Whether a document is in a matter now: `firm`, `proposed` (by the
/// model, not answered yet) or not.
fn membership(conn: &Connection, matter: &str, content: i64) -> Result<Option<String>, String> {
    let firm: Option<String> = conn
        .query_row("SELECT state FROM knowledge_matter_document WHERE matter_id = ?1 AND content_id = ?2", rusqlite::params![matter, content], |row| row.get(0))
        .optional()
        .map_err(sql_err)?;
    if firm.is_some() {
        return Ok(Some("firm".into()));
    }
    let proposals = crate::docbase::knowledge::matters::proposals(conn)?;
    Ok(proposals.get(&content).is_some_and(|list| list.iter().any(|(m, _)| m == matter)).then(|| "proposed".to_string()))
}

fn content_of(item: &CardItem) -> Option<i64> {
    item.key.as_deref().and_then(|k| k.strip_prefix("doc-")).and_then(|n| n.parse().ok())
}

/// What a record shows when deciding about it (as the screen of doubts
/// keeps it): its name, how it is written, its identifiers, its documents.
fn entity_detail(base: &DocBase, id: &str) -> Result<Option<Value>, String> {
    let Some(detail) = base.knowledge_entity(&Visible::All, id)? else { return Ok(None) };
    let identifiers = identifiers_of(&base.conn(), id)?;
    let mut documents: Vec<String> = Vec::new();
    let mut seen = BTreeSet::new();
    for mention in &detail.mentions {
        if seen.insert(mention.content_id) && documents.len() < 8 {
            documents.push(mention.document.clone());
        }
    }
    Ok(Some(json!({
        "key": id,
        "name": detail.name,
        "kind": detail.kind,
        "aliases": detail.aliases.iter().map(|a| a.written.clone()).filter(|w| w != &detail.name).take(6).collect::<Vec<_>>(),
        "identifiers": identifiers,
        "documents": documents,
        "documentCount": seen.len(),
    })))
}

fn plural(n: usize, one: &str, many: &str) -> String {
    if n == 1 {
        format!("1 {one}")
    } else {
        format!("{n} {many}")
    }
}

/// Puts the card as the base is now: each thing's status, the title, the
/// warnings, what blocks it and its fingerprint. `first`: the card is
/// being built, and the ticks follow the statuses; later, what a person
/// unticked stays unticked.
pub fn refresh(base: &DocBase, card: &mut Card, first: bool) -> Result<(), String> {
    let words = super::words::words();
    find(&card.operation).ok_or_else(|| format!("«{}» no es una operación del contrato", card.operation))?;
    let mut print: Vec<String> = Vec::new();
    card.warnings.clear();
    card.blocked = None;
    card.note = None;
    card.open = None;

    // The matter, as it is now.
    let mut matter: Option<MatterLine> = None;
    if let Some(target) = card.target.as_mut() {
        if target.kind == "caso" {
            if let Some(key) = target.key.clone() {
                let line = resolve::matter_line(&base.conn(), &key)?;
                match line {
                    Some(line) if line.state == "active" || card.operation == "caso_decidir" => {
                        target.name = Some(line.name.clone());
                        target.confirmation = Some(line.confirmation.clone());
                        target.status_word = Some(status_word(&line.status).to_string());
                        target.status = "ok".into();
                        print.push(format!("{}|{}|{}|{}|{}", line.key, line.state, line.confirmation, line.status, line.name));
                        card.open = Some(Open { kind: "caso".into(), id: line.key.clone() });
                        matter = Some(line);
                    }
                    _ => {
                        target.status = "not_found".into();
                        print.push(format!("{key}|gone"));
                    }
                }
            } else {
                print.push(format!("{}|{}", target.named, target.status));
            }
        }
    }

    // Each thing.
    for item in card.items.iter_mut() {
        if item.status == "not_found" || item.status == "choose" {
            print.push(format!("{}|{}", item.named, item.status));
            continue;
        }
        let was_unticked = !first && !item.checked && item.status == "ok";
        let (status, reason) = match (card.operation.as_str(), &matter) {
            ("caso_agregar_documentos", Some(m)) => {
                let content = content_of(item).unwrap_or_default();
                let now = membership(&base.conn(), &m.key, content)?;
                print.push(format!("{content}|{}", now.clone().unwrap_or_default()));
                match now.as_deref() {
                    Some("firm") => ("unchanged", Some(format!("ya está en el {}", words.caso))),
                    Some(_) => ("ok", Some("lo había propuesto el modelo".to_string())),
                    None => ("ok", None),
                }
            }
            ("caso_sacar_documentos", Some(m)) => {
                let content = content_of(item).unwrap_or_default();
                let now = membership(&base.conn(), &m.key, content)?;
                print.push(format!("{content}|{}", now.clone().unwrap_or_default()));
                match now.as_deref() {
                    Some("firm") => ("ok", None),
                    Some(_) => ("ok", Some("lo había propuesto el modelo: se descarta la propuesta".to_string())),
                    None => ("unchanged", Some(format!("no está en el {}", words.caso))),
                }
            }
            ("caso_agregar_documentos" | "caso_sacar_documentos", None) => ("blocked", Some(format!("falta el {}", words.caso))),
            ("caso_crear", _) | ("fichas_separar", _) | ("documento_releer", _) if item.key.as_deref().is_some_and(|k| k.starts_with("doc-")) => {
                let content = content_of(item).unwrap_or_default();
                let exists = resolve::document_line(&base.conn(), content)?;
                print.push(format!("{content}|{}", exists.is_some()));
                if exists.is_some() {
                    ("ok", None)
                } else {
                    ("blocked", Some("ya no está en la base".to_string()))
                }
            }
            (_, _) if item.key.as_deref().is_some_and(|k| k.starts_with("ent-")) => {
                let key = item.key.clone().unwrap_or_default();
                let line = resolve::entity_line(&base.conn(), &key)?;
                print.push(format!("{key}|{:?}", line.as_ref().map(|l| (&l.state, &l.merged_into))));
                match line {
                    Some(line) if line.state == "active" => {
                        item.name = Some(line.name.clone());
                        item.detail = entity_detail(base, &key)?;
                        ("ok", None)
                    }
                    Some(line) => match line.merged_into {
                        Some(other) => ("blocked", Some(format!("ya se unió con otra {} ({other})", words.ficha))),
                        None => ("blocked", Some(format!("esa {} ya no está", words.ficha))),
                    },
                    None => ("blocked", Some(format!("esa {} ya no está", words.ficha))),
                }
            }
            _ => ("ok", None),
        };
        item.status = status.to_string();
        item.reason = reason;
        item.checked = match status {
            "ok" => item.required || !was_unticked,
            _ => false,
        };
    }

    // The title, the warnings and what blocks it, by operation.
    let target_name = card.target.as_ref().and_then(|t| t.name.clone()).unwrap_or_else(|| card.target.as_ref().map(|t| t.named.clone()).unwrap_or_default());
    let ticked = card.items.iter().filter(|i| i.checked && i.status == "ok").count();
    let target_status = card.target.as_ref().map(|t| t.status.clone()).unwrap_or_else(|| "ok".into());
    if target_status == "not_found" {
        card.blocked = Some(format!("No encontré el {} «{}» en la base.", words.caso, card.target.as_ref().map(|t| t.named.clone()).unwrap_or_default()));
    } else if target_status == "choose" {
        card.blocked = Some(format!("Elegí cuál es el {}.", words.caso));
    }
    let documentos = |n: usize| plural(n, "documento", "documentos");
    match card.operation.as_str() {
        "caso_agregar_documentos" => {
            card.title = format!("Agregar {} al {} «{}»", documentos(ticked), words.caso, target_name);
            if matter.as_ref().is_some_and(|m| m.confirmation == "proposed") {
                card.warnings.push(format!("El {} era una propuesta: al confirmar, queda confirmado.", words.caso));
            }
            if let Some(m) = matter.as_ref().filter(|m| m.status != "open") {
                card.warnings.push(format!("El {} está {}.", words.caso, status_word(&m.status)));
            }
        }
        "caso_sacar_documentos" => {
            card.title = format!("Sacar {} del {} «{}»", documentos(ticked), words.caso, target_name);
            card.warnings.push("Ninguna pasada de la app los vuelve a poner.".to_string());
            if let Some(m) = matter.as_ref() {
                if m.origin == "proposed" {
                    card.warnings.push(format!("A un {} propuesto no se le puede sacar el último documento: para que no quede, descartalo.", words.caso));
                }
                if m.confirmation == "proposed" {
                    card.warnings.push(format!("El {} era una propuesta: al confirmar, queda confirmado.", words.caso));
                }
            }
        }
        "caso_crear" => {
            card.title = if ticked == 0 {
                format!("Crear el {} «{}», vacío", words.caso, target_name)
            } else {
                format!("Crear el {} «{}» con {}", words.caso, target_name, documentos(ticked))
            };
            if target_name.is_empty() {
                card.blocked = Some(format!("Falta el nombre del {}.", words.caso));
            }
            let same: Option<String> = base
                .conn()
                .query_row("SELECT id FROM knowledge_matter WHERE state = 'active' AND lower(name) = lower(?1)", [&target_name], |row| row.get(0))
                .optional()
                .map_err(sql_err)?;
            print.push(format!("same|{same:?}"));
            if same.is_some() {
                card.warnings.push(format!("Ya hay un {} que se llama así.", words.caso));
            }
        }
        "caso_renombrar" => {
            let new_name = text(&card.input, "nombre").split_whitespace().collect::<Vec<_>>().join(" ");
            card.title = format!("Cambiar el nombre del {} «{}»", words.caso, target_name);
            card.note = Some(format!("Nombre nuevo: «{new_name}»"));
            if matter.as_ref().is_some_and(|m| m.name == new_name) && card.blocked.is_none() {
                card.blocked = Some("Ya se llama así.".to_string());
            }
            card.warnings.push("El nombre queda fijo aunque la app reagrupe.".to_string());
        }
        "caso_decidir" => {
            let decision = text(&card.input, "decision");
            let (verb, already) = match decision.as_str() {
                "confirmar" => ("Confirmar", "confirmed"),
                "descartar" => ("Descartar", "discarded"),
                _ => ("Volver a propuesto", "proposed"),
            };
            card.title = format!("{verb} el {} «{}»", words.caso, target_name);
            if card.blocked.is_none() {
                match matter.as_ref() {
                    Some(m) if m.confirmation == already => {
                        card.blocked = Some(format!("Ya está {}.", match already {
                            "confirmed" => "confirmado",
                            "discarded" => "descartado",
                            _ => "propuesto",
                        }));
                    }
                    Some(m) if m.state != "active" && decision != "volver_a_propuesto" => {
                        card.blocked = Some(format!("Ese {} ya no está activo.", words.caso));
                    }
                    _ => {}
                }
            }
            if decision == "descartar" {
                card.warnings.push(format!("Sus documentos quedan sueltos; el {} sigue en el registro y se puede volver a proponer.", words.caso));
            }
        }
        "caso_cambiar_estado" => {
            let wanted = text(&card.input, "estado");
            let verb = match wanted.as_str() {
                "cerrado" => "Cerrar",
                "archivado" => "Archivar",
                _ => "Reabrir",
            };
            card.title = format!("{verb} el {} «{}»", words.caso, target_name);
            if let Some(m) = matter.as_ref() {
                if status_word(&m.status) == wanted && card.blocked.is_none() {
                    card.blocked = Some(format!("Ya está {wanted}."));
                }
            }
            if wanted != "abierto" {
                card.warnings.push(
                    "Sale de las listas y de Atención, sigue en la búsqueda con la marca y no recibe propuestas nuevas. Se puede reabrir.".to_string(),
                );
            }
        }
        "fichas_unir" | "fichas_separar" => {
            let names: Vec<String> = card.items.iter().map(|i| i.name.clone().unwrap_or_else(|| i.named.clone())).collect();
            let kinds: BTreeSet<String> = card
                .items
                .iter()
                .filter_map(|i| i.detail.as_ref().and_then(|d| d["kind"].as_str().map(str::to_string)))
                .collect();
            let one = names.first().cloned().unwrap_or_default();
            let two = names.get(1).cloned().unwrap_or_default();
            let with_document = card.items.get(1).and_then(|i| i.key.as_deref()).is_some_and(|k| k.starts_with("doc-"));
            card.title = match (card.operation.as_str(), with_document) {
                ("fichas_unir", _) => format!("Unir las {} «{one}» y «{two}»: son la misma", words.fichas),
                (_, true) => format!("Separar a «{one}» en el documento «{two}»: ahí es otra"),
                _ => format!("Separar las {} «{one}» y «{two}»: no son la misma", words.fichas),
            };
            if card.items.len() < 2 {
                card.blocked = Some(format!("Hacen falta las dos {}.", words.fichas));
            } else if card.items.iter().any(|i| i.status == "not_found") {
                card.blocked = Some("No encontré una de las dos en la base.".to_string());
            } else if card.items.iter().any(|i| i.status == "choose") {
                card.blocked = Some("Elegí cuál es cada una.".to_string());
            } else if card.items.iter().any(|i| i.status == "blocked") {
                card.blocked = card.items.iter().find(|i| i.status == "blocked").and_then(|i| i.reason.clone()).map(|r| capitalized(&r) + ".");
            } else if !with_document && card.items[0].key == card.items[1].key {
                card.blocked = Some(format!("Son la misma {}.", words.ficha));
            } else if kinds.len() > 1 {
                card.blocked = Some("Una es una persona y la otra una organización.".to_string());
            }
            if with_document && card.blocked.is_none() {
                let entity = card.items[0].key.clone().unwrap_or_default();
                let content = content_of(&card.items[1]).unwrap_or_default();
                let mentions: i64 = base
                    .conn()
                    .query_row("SELECT count(*) FROM knowledge_mention WHERE entity_id = ?1 AND content_id = ?2", rusqlite::params![entity, content], |row| row.get(0))
                    .map_err(sql_err)?;
                print.push(format!("mentions|{mentions}"));
                if mentions == 0 {
                    card.blocked = Some(format!("Esa {} no aparece en ese documento.", words.ficha));
                }
            }
            if card.operation == "fichas_unir" {
                card.warnings.push("Pasan a ser una, con todos sus documentos; ninguna pasada las vuelve a separar.".to_string());
            } else {
                card.warnings.push("Quedan separadas: ninguna pasada las vuelve a unir.".to_string());
            }
            if let Some(first) = card.items.first().and_then(|i| i.key.clone()).filter(|k| k.starts_with("ent-")) {
                card.open = Some(Open { kind: "ficha".into(), id: first });
            }
        }
        "documento_releer" => {
            card.title = format!("Volver a leer {} con el modelo", documentos(ticked));
            let mut cost = Cost { documents: 0, questions: 0 };
            for item in card.items.iter().filter(|i| i.status == "ok" && i.checked) {
                let Some(location_id) = item.location_id else { continue };
                let plan = base.knowledge_reread_plan(&crate::docbase::knowledge::extract::RereadTarget { location_id: Some(location_id), ..Default::default() })?;
                cost.documents += plan.documents;
                cost.questions += plan.questions;
            }
            print.push(format!("cost|{}|{}", cost.documents, cost.questions));
            card.warnings.push(format!(
                "Cuesta {} al modelo y no se deshace: sus lecturas se rehacen.",
                plural(cost.questions, "pregunta", "preguntas")
            ));
            card.cost = Some(cost);
        }
        "consulta_guardar" => {
            let title = text(&card.input, "titulo");
            card.title = format!("Guardar la consulta «{title}»");
            let answer = text(&card.input, "respuesta");
            let preview: String = answer.chars().take(400).collect();
            card.note = Some(if answer.chars().count() > 400 { format!("{preview}…") } else { preview });
        }
        _ => {}
    }

    // Nothing to do: everything already as asked, or not found.
    let has_items = !card.items.is_empty();
    let needs_items = matches!(card.operation.as_str(), "caso_agregar_documentos" | "caso_sacar_documentos" | "documento_releer");
    if card.blocked.is_none() && needs_items && has_items && !card.items.iter().any(|i| i.status == "ok" || i.status == "choose") {
        card.blocked = Some("No hay nada que hacer: todo ya está como se pide o no está en la base.".to_string());
    }
    if card.blocked.is_none() && needs_items && !has_items {
        card.blocked = Some("Faltan los documentos.".to_string());
    }
    card.second_confirmation = ticked > SECOND_CONFIRMATION_OVER || card.cost.is_some();
    print.sort();
    card.fingerprint = digest(&format!("{}|{}", card.operation, print.join("\n")));
    Ok(())
}

/// What the model gets back when a proposal has nothing to do: what was
/// not found and what was already as asked.
pub fn nothing_to_do(card: &Card) -> Option<String> {
    let words = super::words::words();
    let blocked = card.blocked.as_deref()?;
    let not_found: Vec<&str> = card.items.iter().filter(|i| i.status == "not_found").map(|i| i.named.as_str()).collect();
    let unchanged: Vec<String> = card.items.iter().filter(|i| i.status == "unchanged").map(|i| i.name.clone().unwrap_or_else(|| i.named.clone())).collect();
    let target_missing = card.target.as_ref().is_some_and(|t| t.status == "not_found");
    let choosing = card.target.as_ref().is_some_and(|t| t.status == "choose") || card.items.iter().any(|i| i.status == "choose");
    if choosing {
        return None;
    }
    let mut parts = vec![format!("No hay tarjeta: {blocked}")];
    if target_missing {
        parts.push(format!("El {} «{}» no está en la base.", words.caso, card.target.as_ref().map(|t| t.named.clone()).unwrap_or_default()));
    }
    if !not_found.is_empty() {
        parts.push(format!("No encontré: {}.", not_found.iter().map(|n| format!("«{n}»")).collect::<Vec<_>>().join(", ")));
    }
    if !unchanged.is_empty() {
        parts.push(format!("Ya estaban como se pide: {}.", unchanged.iter().map(|n| format!("«{n}»")).collect::<Vec<_>>().join(", ")));
    }
    parts.push("Decíselo a la persona; no digas que hiciste nada.".to_string());
    Some(parts.join(" "))
}

/// A person chose one of the candidates of a thing (`item`: what the
/// model named, or `target` for the matter).
pub fn choose(card: &mut Card, item: &str, candidate: &str) -> Result<(), String> {
    if item == "target" {
        let target = card.target.as_mut().ok_or_else(|| "Esta tarjeta no tiene a qué elegir".to_string())?;
        let found = target.candidates.iter().find(|c| c.key == candidate).cloned().ok_or_else(|| "Esa opción no está entre las de la tarjeta".to_string())?;
        target.key = Some(found.key);
        target.name = Some(found.name);
        target.status = "ok".into();
        target.candidates.clear();
        return Ok(());
    }
    let entry = card
        .items
        .iter_mut()
        .find(|i| i.named == item && i.status == "choose")
        .ok_or_else(|| "Ese elemento ya no espera una elección".to_string())?;
    let found = entry.candidates.iter().find(|c| c.key == candidate).cloned().ok_or_else(|| "Esa opción no está entre las de la tarjeta".to_string())?;
    entry.key = Some(found.key.clone());
    entry.name = Some(found.name.clone());
    entry.place = found.place.clone();
    entry.location_id = found.location_id;
    entry.date = found.date.clone();
    entry.status = "ok".into();
    entry.checked = true;
    entry.candidates.clear();
    // The same thing chosen twice is one.
    let key = found.key;
    let mut seen = false;
    card.items.retain(|i| {
        if i.key.as_deref() == Some(key.as_str()) && i.status == "ok" {
            if seen {
                return false;
            }
            seen = true;
        }
        true
    });
    Ok(())
}

/// The keys a person left ticked, of those that can be done.
pub fn ticked(card: &Card, chosen: Option<&[String]>) -> Vec<String> {
    card.items
        .iter()
        .filter(|i| i.status == "ok")
        .filter(|i| i.required || chosen.map_or(i.checked, |c| i.key.as_ref().is_some_and(|k| c.contains(k))))
        .filter_map(|i| i.key.clone())
        .collect()
}

/// Turns a card into JSON for the record.
pub fn to_value(card: &Card) -> Value {
    serde_json::to_value(card).unwrap_or(Value::Null)
}

pub fn from_value(value: &Value) -> Result<Card, String> {
    serde_json::from_value(value.clone()).map_err(|e| format!("La tarjeta guardada no se puede leer: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_are_said_in_words() {
        assert_eq!(plural(1, "documento", "documentos"), "1 documento");
        assert_eq!(plural(3, "documento", "documentos"), "3 documentos");
    }
}
