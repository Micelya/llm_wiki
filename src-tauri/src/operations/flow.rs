//! The life of a proposal (3.1, 4.2): the model (or a program) proposes
//! and the app keeps the card in the record as `proposed`; a person
//! chooses among candidates, unticks what they do not want, confirms or
//! cancels; confirming checks first that everything is as the card showed,
//! and with more than ten things asks a second time; what was done is
//! undone from the card or from the screen, with no time limit. Nobody
//! but the app's own window confirms (the commands, D-447): the agent only
//! proposes.

use serde::Serialize;
use serde_json::{json, Value};

use crate::docbase::DocBase;

use super::card::{self, Card, ProposedBy};
use super::log::{self, Origin};
use super::{apply, find, Caller, Kind};

/// What proposing gave.
#[derive(Debug, Clone, PartialEq)]
pub enum Proposal {
    /// A card that waits for a person.
    Card(Card),
    /// Nothing to do (everything not found or already as asked): no card,
    /// and this goes back to the model so it tells the person.
    Nothing(String),
}

fn caller_of(text: &str) -> Caller {
    match text {
        "screen" => Caller::Screen,
        "clipper" => Caller::Clipper,
        t if t.starts_with("program:") => Caller::ProgramChange,
        _ => Caller::Chat,
    }
}

/// Proposes a change of the contract: checks its data, finds what it names
/// in the base, builds its card and keeps it in the record. Changes nothing.
pub fn propose(base: &DocBase, operation: &str, data: &Value, origin: &Origin, attached: &[i64]) -> Result<Proposal, String> {
    let op = find(operation).ok_or_else(|| format!("«{operation}» no es una operación del contrato"))?;
    if op.kind != Kind::Change {
        return Err(format!("«{operation}» solo lee: no se propone, se usa"));
    }
    if !op.ready || !op.callers.contains(&caller_of(&origin.caller)) {
        return Err(format!("«{operation}» todavía no se puede pedir desde aquí"));
    }
    let data = op.check(data)?;
    let proposed_by = ProposedBy { caller: origin.caller.clone(), engine: origin.engine.clone() };
    let mut card = card::build(base, op, &data, origin.request.as_deref(), proposed_by, attached)?;
    if let Some(observation) = card::nothing_to_do(&card) {
        return Ok(Proposal::Nothing(observation));
    }
    let id = log::propose(base, op.name, origin, &data, &card::to_value(&card))?;
    card.id = id;
    log::set_card(base, id, &card::to_value(&card))?;
    Ok(Proposal::Card(card))
}

fn stored(base: &DocBase, id: i64) -> Result<(log::LogEntry, Card), String> {
    let entry = log::get(base, id)?.ok_or_else(|| format!("No hay una propuesta número {id}"))?;
    let mut card = card::from_value(&entry.card)?;
    card.id = id;
    card.state = entry.state.clone();
    Ok((entry, card))
}

fn waits(entry: &log::LogEntry) -> bool {
    matches!(entry.state.as_str(), "proposed" | "failed")
}

/// A card as it is now: one that waits is recomputed with the base.
pub fn current(base: &DocBase, id: i64) -> Result<Card, String> {
    let (entry, mut card) = stored(base, id)?;
    if waits(&entry) {
        card::refresh(base, &mut card, false)?;
        log::set_card(base, id, &card::to_value(&card))?;
    }
    if entry.state == "failed" {
        card.result = entry.error.clone().map(|e| format!("No se pudo hacer: {e}"));
    }
    Ok(card)
}

/// A person chose a candidate («¿Cuál?»).
pub fn choose(base: &DocBase, id: i64, item: &str, candidate: &str) -> Result<Card, String> {
    let (entry, mut card) = stored(base, id)?;
    if !waits(&entry) {
        return Err("Esa propuesta ya no espera una decisión".to_string());
    }
    card::choose(&mut card, item, candidate)?;
    card::refresh(base, &mut card, false)?;
    log::set_card(base, id, &card::to_value(&card))?;
    Ok(card)
}

/// What confirming gave.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Confirmed {
    /// `applied`; `changed` (the base changed since the card was shown:
    /// it is shown again); `needs_second` (more than ten things: confirm
    /// again).
    pub outcome: String,
    pub card: Card,
}

/// A person confirmed a card, with what they left ticked (`chosen`; none:
/// as the card has it) and the fingerprint of the card they saw.
pub fn confirm(base: &DocBase, id: i64, chosen: Option<&[String]>, second: bool, seen: Option<&str>) -> Result<Confirmed, String> {
    let (entry, mut card) = stored(base, id)?;
    if !waits(&entry) {
        return Err("Esa propuesta ya no espera una decisión".to_string());
    }
    if let Some(chosen) = chosen {
        for item in card.items.iter_mut() {
            if item.status == "ok" && !item.required {
                item.checked = item.key.as_ref().is_some_and(|k| chosen.contains(k));
            }
        }
    }
    card::refresh(base, &mut card, false)?;
    if seen.is_some_and(|f| f != card.fingerprint) {
        log::set_card(base, id, &card::to_value(&card))?;
        return Ok(Confirmed { outcome: "changed".into(), card });
    }
    if let Some(why) = card.blocked.clone() {
        return Err(why);
    }
    let keys = card::ticked(&card, None);
    let needs_items = !matches!(card.operation.as_str(), "caso_renombrar" | "caso_decidir" | "caso_cambiar_estado" | "consulta_guardar" | "caso_crear");
    if needs_items && keys.is_empty() {
        return Err("No quedó nada tildado: no hay nada que hacer".to_string());
    }
    if card.second_confirmation && !second {
        log::set_card(base, id, &card::to_value(&card))?;
        return Ok(Confirmed { outcome: "needs_second".into(), card });
    }
    let note = log::note_of(&entry.caller, entry.request.as_deref(), entry.engine.as_deref());
    match apply::apply(base, &card, &keys, &note) {
        Ok(mut undo) => {
            if let Some(created) = undo.get("created").and_then(Value::as_str) {
                card.open = Some(card::Open { kind: "caso".into(), id: created.to_string() });
                if let Some(target) = card.target.as_mut() {
                    target.key = Some(created.to_string());
                }
            }
            // How the base is after it, to see at the time of undoing
            // whether something changed meanwhile.
            let mut after = card.clone();
            card::refresh(base, &mut after, false)?;
            undo["after"] = json!(after.fingerprint);
            card.state = "applied".into();
            card.result = Some(apply::done_text(&card, &keys));
            for item in card.items.iter_mut() {
                item.checked = item.key.as_ref().is_some_and(|k| keys.contains(k));
            }
            log::applied(base, id, &card::to_value(&card), &json!(keys), &undo)?;
            Ok(Confirmed { outcome: "applied".into(), card })
        }
        Err(error) => {
            log::failed(base, id, &error)?;
            Err(error)
        }
    }
}

/// A person cancelled a card: nothing was done.
pub fn cancel(base: &DocBase, id: i64) -> Result<Card, String> {
    let (entry, mut card) = stored(base, id)?;
    if !waits(&entry) {
        return Err("Esa propuesta ya no espera una decisión".to_string());
    }
    card.state = "cancelled".into();
    card.result = Some("Cancelado: no se hizo nada.".into());
    log::cancelled(base, id, &card::to_value(&card))?;
    Ok(card)
}

/// What undoing gave.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Undone {
    /// `undone`, or `changed`: something changed since it was done, and
    /// undoing waits for the person to confirm it.
    pub outcome: String,
    pub card: Card,
}

/// Undoes what a person confirmed. `force`: the person confirmed it after
/// being told something changed since.
pub fn undo(base: &DocBase, id: i64, force: bool) -> Result<Undone, String> {
    let (entry, mut card) = stored(base, id)?;
    if entry.state != "applied" {
        return Err("Eso no está hecho: no hay nada que deshacer".to_string());
    }
    if !card.undoable {
        return Err(card.undo_note.clone().unwrap_or_else(|| "Esto no se deshace".to_string()));
    }
    let undo = entry.undo.clone().unwrap_or(Value::Null);
    let mut now = card.clone();
    card::refresh(base, &mut now, false)?;
    let after = undo.get("after").and_then(Value::as_str).unwrap_or_default();
    if !force && !after.is_empty() && now.fingerprint != after {
        card.warnings.insert(0, "Algo cambió desde que se hizo: deshacer vuelve a como estaba antes, y puede pisar ese cambio.".to_string());
        return Ok(Undone { outcome: "changed".into(), card });
    }
    apply::undo(base, &card, &undo)?;
    card.state = "undone".into();
    card.result = Some("Deshecho: volvió a como estaba.".into());
    log::undone(base, id, &card::to_value(&card))?;
    Ok(Undone { outcome: "undone".into(), card })
}

/// A button of a screen: the person is who decides, so it is applied at
/// once, and kept in the record like the others (it can be undone).
pub fn apply_now(base: &DocBase, operation: &str, data: &Value) -> Result<Card, String> {
    let origin = Origin { caller: "screen".into(), ..Default::default() };
    let op = find(operation).ok_or_else(|| format!("«{operation}» no es una operación del contrato"))?;
    if !op.ready || !op.callers.contains(&Caller::Screen) {
        return Err(format!("«{operation}» no se hace desde una pantalla"));
    }
    let data = op.check(data)?;
    let proposed_by = ProposedBy { caller: "screen".into(), engine: None };
    let mut card = card::build(base, op, &data, None, proposed_by, &[])?;
    if let Some(why) = card.blocked.clone() {
        return Err(why);
    }
    let id = log::propose(base, op.name, &origin, &data, &card::to_value(&card))?;
    card.id = id;
    log::set_card(base, id, &card::to_value(&card))?;
    Ok(confirm(base, id, None, true, None)?.card)
}
