//! The local API (6 and 11.8 of the contract's design), for programs and
//! scripts of the machine, with the token of a program:
//!
//! - `GET /api/v1/operaciones`: the operations this program can use, with
//!   what each one asks for (its JSON Schema) and an example.
//! - `POST /api/v1/operaciones/<nombre>`, with the data as the body: a
//!   reading answers `{ "ok": true, "resultado": … }`; a change waits for
//!   a person and answers `{ "ok": true, "pedido": 12, "estado":
//!   "pendiente" }`.
//! - `GET /api/v1/pedidos/<número>`: what became of a change it asked for.
//!
//! Errors are in Spanish: `{ "ok": false, "error": "…" }`.

use serde_json::{json, Value};

use crate::docbase::DocBase;

use super::calls::{self, Called, WAITS};
use super::programs::Program;

const PREFIX: &str = "/api/v1";

/// What a request of the API gets: its status and its body. `proposed`
/// is told when a change was left waiting (Atención refreshes).
pub fn route(method: &str, path: &str, body: &str, base: &DocBase, program: &Program, proposed: &dyn Fn()) -> (u16, String) {
    let Some(rest) = path.strip_prefix(PREFIX) else {
        return (404, super::error_json("No existe: la API de Micelya está en /api/v1"));
    };
    let rest = rest.trim_end_matches('/');
    match (method, rest) {
        ("GET", "" | "/operaciones") => {
            let _ = super::programs::note_use(base, &program.id, None);
            ok(json!({
                "programa": { "nombre": program.name, "permiso": program.permission.words() },
                "operaciones": calls::listing(program.permission),
            }))
        }
        ("POST", rest) if rest.starts_with("/operaciones/") => {
            let name = &rest["/operaciones/".len()..];
            let data: Value = if body.trim().is_empty() {
                json!({})
            } else {
                match serde_json::from_str(body) {
                    Ok(data) => data,
                    Err(error) => return (400, super::error_json(&format!("Los datos no son JSON: {error}"))),
                }
            };
            match calls::call(base, program, name, &data) {
                Ok(Called::Read(found)) => ok(json!({ "resultado": found })),
                Ok(Called::Proposed { id, title }) => {
                    proposed();
                    (202, json!({ "ok": true, "pedido": id, "estado": "pendiente", "titulo": title, "mensaje": WAITS }).to_string())
                }
                Ok(Called::Nothing(why)) => ok(json!({ "pedido": null, "estado": "nada_que_hacer", "mensaje": why })),
                Err(refused) => (refused.status, super::error_json(&refused.message)),
            }
        }
        ("GET", rest) if rest.starts_with("/pedidos/") => {
            let Ok(id) = rest["/pedidos/".len()..].parse::<i64>() else {
                return (400, super::error_json("El número de pedido no es un número"));
            };
            match calls::state_of(base, program, id) {
                Ok(state) => ok(state),
                Err(refused) => (refused.status, super::error_json(&refused.message)),
            }
        }
        ("GET" | "POST", _) => (404, super::error_json("No existe: mirá GET /api/v1/operaciones")),
        _ => (405, super::error_json("Ese método no se usa aquí")),
    }
}

fn ok(mut value: Value) -> (u16, String) {
    if let Some(map) = value.as_object_mut() {
        map.insert("ok".into(), json!(true));
    }
    (200, value.to_string())
}
