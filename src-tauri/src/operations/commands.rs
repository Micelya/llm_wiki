//! The commands the app's window uses for the contract (11.7). Like every
//! command, only the app's own window reaches them (`only_app_webview`,
//! D-447): confirming a card is a person in the app, never the model, a
//! program or a skill.

use serde_json::Value;
use tauri::{Emitter, State};

use crate::docbase::commands::{DocBaseState, CHANGED_EVENT};

use super::card::Card;
use super::flow::{self, Confirmed, Undone};
use super::log::LogEntry;
use super::{contract, words::words, OperationView};

/// The contract, with its words, for Ajustes and the tests.
#[tauri::command]
pub fn operations_contract() -> Vec<OperationView> {
    contract(words())
}

/// A card as it is now.
#[tauri::command(async)]
pub fn operations_card(state: State<'_, DocBaseState>, id: i64) -> Result<Card, String> {
    flow::current(&*state.shared()?, id)
}

/// A person chose a candidate of a card («¿Cuál?»; `item` is what the
/// model named, or `target`).
#[tauri::command(async)]
pub fn operations_choose(state: State<'_, DocBaseState>, id: i64, item: String, candidate: String) -> Result<Card, String> {
    flow::choose(&*state.shared()?, id, &item, &candidate)
}

/// A person confirmed a card with what they left ticked.
#[tauri::command(async)]
pub fn operations_confirm(
    app: tauri::AppHandle,
    state: State<'_, DocBaseState>,
    id: i64,
    chosen: Option<Vec<String>>,
    second_confirmation: Option<bool>,
    fingerprint: Option<String>,
) -> Result<Confirmed, String> {
    let confirmed = flow::confirm(&*state.shared()?, id, chosen.as_deref(), second_confirmation.unwrap_or(false), fingerprint.as_deref())?;
    if confirmed.outcome == "applied" {
        if confirmed.card.operation == "documento_releer" {
            state.request_asking();
        }
        let _ = app.emit(CHANGED_EVENT, ());
    }
    Ok(confirmed)
}

/// A person cancelled a card.
#[tauri::command(async)]
pub fn operations_cancel(state: State<'_, DocBaseState>, id: i64) -> Result<Card, String> {
    flow::cancel(&*state.shared()?, id)
}

/// A person undid what a card did (`force`: after being told something
/// changed since).
#[tauri::command(async)]
pub fn operations_undo(app: tauri::AppHandle, state: State<'_, DocBaseState>, id: i64, force: Option<bool>) -> Result<Undone, String> {
    let undone = flow::undo(&*state.shared()?, id, force.unwrap_or(false))?;
    if undone.outcome == "undone" {
        let _ = app.emit(CHANGED_EVENT, ());
    }
    Ok(undone)
}

/// A button of a screen does an operation of the contract: at once,
/// because the person is who decides, and kept in the record.
#[tauri::command(async)]
pub fn operations_apply(app: tauri::AppHandle, state: State<'_, DocBaseState>, operation: String, data: Value) -> Result<Card, String> {
    let card = flow::apply_now(&*state.shared()?, &operation, &data)?;
    if card.operation == "documento_releer" {
        state.request_asking();
    }
    let _ = app.emit(CHANGED_EVENT, ());
    Ok(card)
}

/// The record of operations, the newest first.
#[tauri::command(async)]
pub fn operations_log(state: State<'_, DocBaseState>, filter: Option<String>, limit: Option<usize>) -> Result<Vec<LogEntry>, String> {
    super::log::list(&*state.shared()?, filter.as_deref(), limit.unwrap_or(100).min(1_000))
}
