//! The record of operations (`operation_log`, 7 and 11.5): every change
//! proposed through the contract, with the request of the person, the
//! card as it was seen, what was left ticked, who proposed it, when,
//! whether it was confirmed, cancelled or undone, and what undoing it
//! needs. Nothing is deleted from it.

use rusqlite::{params, OptionalExtension, Row};
use serde::Serialize;
use serde_json::Value;

use crate::docbase::{now_ms, sql_err, DocBase};

/// One entry of the record.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogEntry {
    pub id: i64,
    pub operation: String,
    pub caller: String,
    pub request: Option<String>,
    pub conversation: Option<String>,
    pub engine: Option<String>,
    pub input: Value,
    pub card: Value,
    pub chosen: Option<Value>,
    /// `proposed`, `applied`, `cancelled`, `undone`, `failed` or `expired`.
    pub state: String,
    pub undo: Option<Value>,
    pub error: Option<String>,
    pub proposed_at: i64,
    pub decided_at: Option<i64>,
    pub undone_at: Option<i64>,
}

/// Who proposed it and why, for a new entry.
#[derive(Debug, Clone, Default)]
pub struct Origin {
    pub caller: String,
    pub request: Option<String>,
    pub conversation: Option<String>,
    pub engine: Option<String>,
}

const COLUMNS: &str = "id, operation, caller, request, conversation, engine, input, card, chosen, state, undo, error, proposed_at, decided_at, undone_at";

fn json_of(text: Option<String>) -> Option<Value> {
    text.and_then(|t| serde_json::from_str(&t).ok())
}

fn entry(row: &Row) -> rusqlite::Result<LogEntry> {
    Ok(LogEntry {
        id: row.get(0)?,
        operation: row.get(1)?,
        caller: row.get(2)?,
        request: row.get(3)?,
        conversation: row.get(4)?,
        engine: row.get(5)?,
        input: json_of(row.get(6)?).unwrap_or(Value::Null),
        card: json_of(row.get(7)?).unwrap_or(Value::Null),
        chosen: json_of(row.get(8)?),
        state: row.get(9)?,
        undo: json_of(row.get(10)?),
        error: row.get(11)?,
        proposed_at: row.get(12)?,
        decided_at: row.get(13)?,
        undone_at: row.get(14)?,
    })
}

/// Keeps a proposal; returns its number.
pub fn propose(base: &DocBase, operation: &str, origin: &Origin, input: &Value, card: &Value) -> Result<i64, String> {
    let conn = base.conn();
    conn.execute(
        "INSERT INTO operation_log (operation, caller, request, conversation, engine, input, card, state, proposed_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'proposed', ?8)",
        params![operation, origin.caller, origin.request, origin.conversation, origin.engine, input.to_string(), card.to_string(), now_ms()],
    )
    .map_err(sql_err)?;
    Ok(conn.last_insert_rowid())
}

pub fn get(base: &DocBase, id: i64) -> Result<Option<LogEntry>, String> {
    let conn = base.conn();
    conn.query_row(&format!("SELECT {COLUMNS} FROM operation_log WHERE id = ?1"), [id], entry).optional().map_err(sql_err)
}

/// The card as it is now shown (after a choice, or recomputed).
pub fn set_card(base: &DocBase, id: i64, card: &Value) -> Result<(), String> {
    base.conn().execute("UPDATE operation_log SET card = ?2 WHERE id = ?1", params![id, card.to_string()]).map_err(sql_err)?;
    Ok(())
}

/// A proposal a person confirmed and the app applied.
pub fn applied(base: &DocBase, id: i64, card: &Value, chosen: &Value, undo: &Value) -> Result<(), String> {
    let changed = base
        .conn()
        .execute(
            "UPDATE operation_log SET state = 'applied', card = ?2, chosen = ?3, undo = ?4, decided_at = ?5, error = NULL
             WHERE id = ?1 AND state IN ('proposed', 'failed')",
            params![id, card.to_string(), chosen.to_string(), undo.to_string(), now_ms()],
        )
        .map_err(sql_err)?;
    if changed == 0 {
        return Err("Esa propuesta ya no espera una decisión".to_string());
    }
    Ok(())
}

/// A proposal that could not be applied: it stays, with why.
pub fn failed(base: &DocBase, id: i64, error: &str) -> Result<(), String> {
    base.conn()
        .execute("UPDATE operation_log SET state = 'failed', error = ?2 WHERE id = ?1 AND state IN ('proposed', 'failed')", params![id, error])
        .map_err(sql_err)?;
    Ok(())
}

/// A proposal a person cancelled.
pub fn cancelled(base: &DocBase, id: i64, card: &Value) -> Result<(), String> {
    let changed = base
        .conn()
        .execute(
            "UPDATE operation_log SET state = 'cancelled', card = ?2, decided_at = ?3 WHERE id = ?1 AND state IN ('proposed', 'failed')",
            params![id, card.to_string(), now_ms()],
        )
        .map_err(sql_err)?;
    if changed == 0 {
        return Err("Esa propuesta ya no espera una decisión".to_string());
    }
    Ok(())
}

/// What a person confirmed, undone.
pub fn undone(base: &DocBase, id: i64, card: &Value) -> Result<(), String> {
    let changed = base
        .conn()
        .execute(
            "UPDATE operation_log SET state = 'undone', card = ?2, undone_at = ?3 WHERE id = ?1 AND state = 'applied'",
            params![id, card.to_string(), now_ms()],
        )
        .map_err(sql_err)?;
    if changed == 0 {
        return Err("Eso no está hecho: no hay nada que deshacer".to_string());
    }
    Ok(())
}

/// The latest entries, the newest first; `state` narrows them.
pub fn list(base: &DocBase, state: Option<&str>, limit: usize) -> Result<Vec<LogEntry>, String> {
    let conn = base.conn();
    let mut statement = conn
        .prepare(&format!("SELECT {COLUMNS} FROM operation_log WHERE (?1 IS NULL OR state = ?1) ORDER BY id DESC LIMIT ?2"))
        .map_err(sql_err)?;
    let rows = statement.query_map(params![state, limit as i64], entry).map_err(sql_err)?;
    rows.collect::<Result<_, _>>().map_err(sql_err)
}

/// The note of the decisions a change makes about records and matters:
/// who asked for it (`pedido en el chat: «…»`).
pub fn note_of(origin_caller: &str, request: Option<&str>) -> String {
    if origin_caller == "chat" {
        let request: String = request.unwrap_or_default().chars().take(200).collect();
        format!("pedido en el chat: «{}»", request.trim())
    } else if let Some(program) = origin_caller.strip_prefix("program:") {
        format!("pedido por {program}")
    } else {
        "desde la pantalla".to_string()
    }
}
