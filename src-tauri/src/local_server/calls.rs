//! What a program asks of the contract, the same through the local API and
//! through the MCP (6.3 and 6.4 of the contract's design). A reading is
//! answered; a change is only proposed: it is kept in the record of
//! operations as `proposed`, with the program as who asked, and waits in
//! Atención for a person to confirm it with the same card as the chat's.
//! Nothing a program asks changes the base by itself.

use serde_json::{json, Value};

use crate::docbase::{now_ms, DocBase};
use crate::operations::log::{self, Origin};
use crate::operations::read::{self, ReadContext};
use crate::operations::{find, flow, offered_to, Caller, Kind, Operation, Proposal};

use super::programs::{self, Permission, Program};

/// The caller of the contract a program is, by its permission.
fn caller_of(permission: Permission) -> Caller {
    match permission {
        Permission::Read => Caller::ProgramRead,
        Permission::Change => Caller::ProgramChange,
    }
}

/// The operations a program can use now.
pub fn offered(permission: Permission) -> Vec<&'static Operation> {
    offered_to(caller_of(permission)).collect()
}

/// What a call gave.
#[derive(Debug, Clone, PartialEq)]
pub enum Called {
    /// What a reading found.
    Read(Value),
    /// A change kept as a request that waits for a person.
    Proposed { id: i64, title: String },
    /// A change with nothing to do: what was not found, or is already so.
    Nothing(String),
}

/// Why a call was not made: how the API answers it, and why in words.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refused {
    pub status: u16,
    pub message: String,
}

fn refused(status: u16, message: impl Into<String>) -> Refused {
    Refused { status, message: message.into() }
}

/// The record's name of a program.
pub fn caller_name(program: &Program) -> String {
    format!("program:{}", program.id)
}

/// The words a program gets when its change waits.
pub const WAITS: &str = "Quedó pendiente de confirmación en Micelya (Atención): una persona lo revisa y decide. Todavía no se hizo nada.";

/// A program asks for an operation of the contract with its data.
pub fn call(base: &DocBase, program: &Program, name: &str, data: &Value) -> Result<Called, Refused> {
    let Some(op) = find(name) else {
        return Err(refused(404, format!("«{name}» no es una operación de Micelya")));
    };
    let caller = caller_of(program.permission);
    let for_programs = op.callers.iter().any(|c| matches!(c, Caller::ProgramRead | Caller::ProgramChange));
    if !op.ready || !for_programs {
        return Err(refused(403, format!("«{name}» no se puede pedir desde otro programa")));
    }
    if !op.callers.contains(&caller) {
        return Err(refused(
            403,
            format!(
                "«{}» solo tiene permiso para {}. Para que pueda pedir cambios, en Micelya: Ajustes → Programas conectados → «Leer y pedir cambios».",
                program.name,
                program.permission.words()
            ),
        ));
    }
    let data = op.check(data).map_err(|why| refused(400, why))?;
    if let Err(error) = programs::note_use(base, &program.id, Some(op.name)) {
        eprintln!("[local-server] the use of a program could not be noted: {error}");
    }
    match op.kind {
        Kind::Read => {
            if op.name == "estado_pedido" {
                let id = data.get("pedido").and_then(Value::as_i64).unwrap_or_default();
                return state_of(base, program, id).map(Called::Read);
            }
            read::read(base, op.name, &data, &ReadContext::default()).map(Called::Read).map_err(|why| refused(400, why))
        }
        Kind::Change => {
            let origin = Origin { caller: caller_name(program), engine: Some(program.name.clone()), ..Default::default() };
            match flow::propose(base, op.name, &data, &origin, &[]).map_err(|why| refused(400, why))? {
                Proposal::Card(card) => Ok(Called::Proposed { id: card.id, title: card.title }),
                Proposal::Nothing(observation) => Ok(Called::Nothing(observation)),
            }
        }
    }
}

/// A request of this program, in words: `pendiente`, `hecho`… Only its own.
pub fn state_of(base: &DocBase, program: &Program, id: i64) -> Result<Value, Refused> {
    if let Err(error) = log::expire_program_requests(base, now_ms()) {
        eprintln!("[local-server] the old requests could not expire: {error}");
    }
    let entry = log::get(base, id).map_err(|why| refused(500, why))?;
    let Some(entry) = entry.filter(|e| e.caller == caller_name(program)) else {
        return Err(refused(404, format!("No hay un pedido número {id} de este programa")));
    };
    let state = match entry.state.as_str() {
        "proposed" | "failed" => "pendiente",
        "applied" => "hecho",
        "cancelled" => "cancelado",
        "undone" => "deshecho",
        _ => "vencido",
    };
    let title = entry.card.get("title").and_then(Value::as_str).unwrap_or_default().to_string();
    let result = entry.card.get("result").and_then(Value::as_str).map(str::to_string);
    Ok(json!({ "pedido": id, "operacion": entry.operation, "titulo": title, "estado": state, "resultado": result }))
}

/// The operations of a program, as the API and the MCP list them.
pub fn listing(permission: Permission) -> Vec<Value> {
    let words = crate::operations::words::words();
    offered(permission)
        .into_iter()
        .map(|op| {
            json!({
                "nombre": op.name,
                "resumen": words.fill(op.summary),
                "cambia": op.kind == Kind::Change,
                "esquema": op.input_schema(words),
                "ejemplo": serde_json::from_str::<Value>(op.example).unwrap_or(Value::Null),
            })
        })
        .collect()
}
