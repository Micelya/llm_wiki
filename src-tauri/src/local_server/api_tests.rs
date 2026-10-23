//! A program through the local API, on a base of invented documents (R-24:
//! every name here is made up): it only answers with a token; a program
//! that only reads cannot ask for a change; one that may, leaves it waiting
//! for a person, who confirms it with the same card as the chat's.

use std::sync::atomic::Ordering;
use std::sync::Arc;

use serde_json::{json, Value};

use super::{context_with, Probe, Running};
use crate::docbase::knowledge::tests::{base_with, location};
use crate::docbase::testing::TempDir;
use crate::docbase::{DocBase, Visible};
use crate::local_server::programs::{self, Permission};
use crate::operations::{flow, log};
use crate::secrets::fake::FakeKeychain;

pub(super) const FILES: &[(&str, &str)] = &[
    ("Carta documento 03-12.txt", "Carta documento del 12 de marzo de Ana Prueba a Ejemplo SA por el depósito de la calle Falsa."),
    ("Carta documento 03-19.txt", "Carta documento del 19 de marzo de Ejemplo SA a Ana Prueba, en respuesta a la del 12."),
    ("Contrato de locación.txt", "Contrato de locación entre Ana Prueba y Ejemplo SA del depósito de la calle Falsa."),
];

pub(super) fn key(base: &DocBase, name: &str) -> String {
    let location = location(base, name);
    format!("doc-{}", base.document(&Visible::All, location).unwrap().unwrap().row.content_id.unwrap())
}

pub(super) fn matter_documents(base: &DocBase, matter: &str) -> usize {
    base.conn().query_row("SELECT count(*) FROM knowledge_matter_document WHERE matter_id = ?1", [matter], |row| row.get::<_, i64>(0)).unwrap() as usize
}

/// A base with a case of one document, a program that only reads and one
/// that may ask for changes, and the server over it.
pub(super) struct World {
    pub _dir: TempDir,
    pub _clips: TempDir,
    pub base: Arc<DocBase>,
    pub matter: String,
    pub reader: String,
    pub changer: String,
    pub changer_id: String,
    pub probe: Probe,
    pub server: Running,
}

pub(super) fn world() -> World {
    let (dir, base) = base_with(FILES);
    let contract = key(&base, "Contrato de locación.txt").replace("doc-", "content:");
    let matter = base.knowledge_create_matter("Prueba c/ Ejemplo SA", &[contract]).unwrap();
    let keychain = FakeKeychain::default();
    let reader = programs::add(&base, &keychain, "Asistente de prueba", Permission::Read).unwrap();
    let changer = programs::add(&base, &keychain, "Script de prueba", Permission::Change).unwrap();
    let base = Arc::new(base);
    let clips = TempDir::new();
    let probe = Probe::default();
    let server = Running::start(context_with(base.clone(), clips.path().to_path_buf(), probe.clone()));
    World {
        _dir: dir,
        _clips: clips,
        base,
        matter,
        reader: reader.token,
        changer: changer.token,
        changer_id: changer.program.id,
        probe,
        server,
    }
}

pub(super) fn bearer(token: &str) -> String {
    format!("Authorization: Bearer {token}\r\n")
}

fn body(text: &str) -> Value {
    serde_json::from_str(text).unwrap_or_else(|e| panic!("not JSON ({e}): {text}"))
}

#[test]
fn without_the_token_of_a_program_the_api_answers_nothing() {
    let w = world();
    let (status, text) = w.server.get("/api/v1/operaciones", "");
    assert_eq!(status, 401, "{text}");
    assert!(body(&text)["error"].as_str().unwrap().contains("Ajustes → Programas conectados"));
    let (status, _) = w.server.get("/api/v1/operaciones", &bearer("mcy_inventado"));
    assert_eq!(status, 401);
    let (status, _) = w.server.post("/api/v1/operaciones/buscar", &bearer("mcy_inventado"), "application/json", r#"{"consulta":"carta"}"#);
    assert_eq!(status, 401);
    // Not even with the token, from a web page.
    let (status, _) = w.server.get("/api/v1/operaciones", &format!("Origin: https://sitio.invalid\r\n{}", bearer(&w.reader)));
    assert_eq!(status, 403);
    // Nor addressed to another name of the machine.
    let (status, _) = w.server.send(&format!("GET /api/v1/operaciones HTTP/1.1\r\nHost: sitio.invalid:{}\r\n{}Connection: close\r\n\r\n", w.server.port, bearer(&w.reader)));
    assert_eq!(status, 421);
    // With the programs off, nobody; the clipper keeps working.
    w.probe.off.store(true, Ordering::Relaxed);
    let (status, _) = w.server.get("/api/v1/operaciones", &bearer(&w.reader));
    assert_eq!(status, 503);
    let (status, _) = w.server.get("/status", "");
    assert_eq!(status, 200);
}

#[test]
fn a_program_that_only_reads_sees_and_uses_only_the_readings() {
    let w = world();
    let (status, text) = w.server.get("/api/v1/operaciones", &bearer(&w.reader));
    assert_eq!(status, 200, "{text}");
    let listing = body(&text);
    assert_eq!(listing["programa"]["permiso"], "solo leer");
    let names: Vec<&str> = listing["operaciones"].as_array().unwrap().iter().map(|o| o["nombre"].as_str().unwrap()).collect();
    assert!(names.contains(&"buscar") && names.contains(&"ver_caso") && names.contains(&"estado_pedido"), "{names:?}");
    assert!(!names.contains(&"caso_agregar_documentos") && !names.contains(&"recorte_guardar"), "{names:?}");
    assert!(listing["operaciones"][0]["esquema"]["properties"].is_object());

    // A reading, with invented data.
    let (status, text) = w.server.post("/api/v1/operaciones/buscar", &bearer(&w.reader), "application/json", r#"{"consulta":"carta documento","cantidad":5}"#);
    assert_eq!(status, 200, "{text}");
    let found = body(&text);
    let results = found["resultado"]["resultados"].as_array().unwrap();
    assert!(results.iter().any(|r| r["nombre"] == "Carta documento 03-12.txt"), "{found}");
    let (status, text) = w.server.post("/api/v1/operaciones/ver_caso", &bearer(&w.reader), "application/json", r#"{"caso":"Prueba"}"#);
    assert_eq!(status, 200, "{text}");
    assert_eq!(body(&text)["resultado"]["nombre"], "Prueba c/ Ejemplo SA");

    // A change: refused, saying how to allow it.
    let change = json!({ "caso": w.matter, "documentos": [key(&w.base, "Carta documento 03-12.txt")] }).to_string();
    let (status, text) = w.server.post("/api/v1/operaciones/caso_agregar_documentos", &bearer(&w.reader), "application/json", &change);
    assert_eq!(status, 403, "{text}");
    assert!(body(&text)["error"].as_str().unwrap().contains("Leer y pedir cambios"));
    assert_eq!(log::list(&w.base, None, 10).unwrap().len(), 0);

    // What is not in the contract, data that do not fit, a body that is not JSON.
    let (status, _) = w.server.post("/api/v1/operaciones/borrar_todo", &bearer(&w.reader), "application/json", "{}");
    assert_eq!(status, 404);
    let (status, text) = w.server.post("/api/v1/operaciones/buscar", &bearer(&w.reader), "application/json", r#"{"cantidad":"muchos"}"#);
    assert_eq!(status, 400, "{text}");
    let (status, _) = w.server.post("/api/v1/operaciones/buscar", &bearer(&w.reader), "text/plain", "consulta=carta");
    assert_eq!(status, 415);
    // Its uses are counted, without what it read.
    let uses: i64 = w.base.conn().query_row("SELECT sum(count) FROM local_program_use WHERE operation = 'buscar'", [], |row| row.get(0)).unwrap();
    assert!(uses >= 1);
}

#[test]
fn a_change_a_program_asks_for_waits_for_a_person_and_is_done_only_when_confirmed() {
    let w = world();
    let added = key(&w.base, "Carta documento 03-12.txt");
    let change = json!({ "caso": "Prueba c/ Ejemplo SA", "documentos": [added] }).to_string();
    let (status, text) = w.server.post("/api/v1/operaciones/caso_agregar_documentos", &bearer(&w.changer), "application/json", &change);
    assert_eq!(status, 202, "{text}");
    let answer = body(&text);
    assert_eq!(answer["estado"], "pendiente");
    assert!(answer["mensaje"].as_str().unwrap().contains("Atención"));
    let id = answer["pedido"].as_i64().unwrap();
    // Nothing was done: it waits in the record, as asked by the program.
    assert_eq!(matter_documents(&w.base, &w.matter), 1);
    let entry = log::get(&w.base, id).unwrap().unwrap();
    assert_eq!(entry.state, "proposed");
    assert_eq!(entry.caller, format!("program:{}", w.changer_id));
    assert_eq!(entry.engine.as_deref(), Some("Script de prueba"));
    assert_eq!(log::program_requests(&w.base).unwrap().len(), 1);
    assert_eq!(w.probe.proposed.load(Ordering::Relaxed), 1);
    // The program asks what became of it; another program cannot.
    let (status, text) = w.server.get(&format!("/api/v1/pedidos/{id}"), &bearer(&w.changer));
    assert_eq!(status, 200, "{text}");
    assert_eq!(body(&text)["estado"], "pendiente");
    let (status, _) = w.server.get(&format!("/api/v1/pedidos/{id}"), &bearer(&w.reader));
    assert_eq!(status, 404);

    // A person confirms it in the app, with the card.
    let card = flow::current(&w.base, id).unwrap();
    assert_eq!(card.title, "Agregar 1 documento al caso «Prueba c/ Ejemplo SA»");
    assert_eq!(card.proposed_by.caller, format!("program:{}", w.changer_id));
    let done = flow::confirm(&w.base, id, None, false, Some(&card.fingerprint)).unwrap();
    assert_eq!(done.outcome, "applied");
    assert_eq!(matter_documents(&w.base, &w.matter), 2);
    let (_, text) = w.server.post("/api/v1/operaciones/estado_pedido", &bearer(&w.changer), "application/json", &format!(r#"{{"pedido":{id}}}"#));
    assert_eq!(body(&text)["resultado"]["estado"], "hecho", "{text}");
    assert!(log::program_requests(&w.base).unwrap().is_empty());

    // Asked again, there is nothing to do: no request is left.
    let (status, text) = w.server.post("/api/v1/operaciones/caso_agregar_documentos", &bearer(&w.changer), "application/json", &change);
    assert_eq!(status, 200, "{text}");
    assert_eq!(body(&text)["estado"], "nada_que_hacer");
    assert!(log::program_requests(&w.base).unwrap().is_empty());
}

#[test]
fn a_request_nobody_decided_in_seven_days_expires() {
    let w = world();
    let change = json!({ "caso": w.matter, "nombre": "Prueba c/ Ejemplo SA (renombrado)" }).to_string();
    let (status, text) = w.server.post("/api/v1/operaciones/caso_renombrar", &bearer(&w.changer), "application/json", &change);
    assert_eq!(status, 202, "{text}");
    let id = body(&text)["pedido"].as_i64().unwrap();
    let eight_days = 8 * 24 * 60 * 60 * 1000;
    w.base.conn().execute("UPDATE operation_log SET proposed_at = proposed_at - ?2 WHERE id = ?1", rusqlite::params![id, eight_days]).unwrap();
    let (_, text) = w.server.get(&format!("/api/v1/pedidos/{id}"), &bearer(&w.changer));
    assert_eq!(body(&text)["estado"], "vencido", "{text}");
    assert_eq!(log::get(&w.base, id).unwrap().unwrap().state, "expired");
    assert!(flow::confirm(&w.base, id, None, true, None).is_err());
}
