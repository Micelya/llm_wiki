//! An assistant through the MCP, as Claude Code talks to it, on a base of
//! invented documents (R-24): the same operations of the contract as the
//! chat and the local API, by the program's permission.

use std::sync::atomic::Ordering;

use serde_json::{json, Value};

use super::programs::{bearer, key, matter_documents, world};
use crate::operations::{flow, log, offered_to, Caller};

fn rpc(w: &super::programs::World, token: &str, message: Value) -> (u16, Value) {
    let (status, text) = w.server.post("/mcp", &format!("{}Accept: application/json, text/event-stream\r\n", bearer(token)), "application/json", &message.to_string());
    let value = if text.trim().is_empty() { Value::Null } else { serde_json::from_str(&text).unwrap_or_else(|e| panic!("not JSON ({e}): {text}")) };
    (status, value)
}

#[test]
fn an_assistant_connects_lists_the_readings_and_reads_with_invented_data() {
    let w = world();
    // Without the token, nothing.
    let (status, _) = w.server.post("/mcp", "", "application/json", r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#);
    assert_eq!(status, 401);

    let (status, init) = rpc(&w, &w.reader, json!({ "jsonrpc": "2.0", "id": 1, "method": "initialize", "params": { "protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": { "name": "prueba", "version": "1" } } }));
    assert_eq!(status, 200);
    assert_eq!(init["result"]["protocolVersion"], "2025-06-18");
    assert_eq!(init["result"]["serverInfo"]["name"], "micelya");
    assert!(init["result"]["capabilities"]["tools"].is_object());
    assert!(init["result"]["instructions"].as_str().unwrap().contains("solo puede leer"));
    // A version it does not know: it offers its newest.
    let (_, other) = rpc(&w, &w.reader, json!({ "jsonrpc": "2.0", "id": 2, "method": "initialize", "params": { "protocolVersion": "1999-01-01" } }));
    assert_eq!(other["result"]["protocolVersion"], "2025-06-18");
    // A notification has no answer.
    let (status, empty) = rpc(&w, &w.reader, json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }));
    assert_eq!(status, 202);
    assert_eq!(empty, Value::Null);
    let (_, pong) = rpc(&w, &w.reader, json!({ "jsonrpc": "2.0", "id": 3, "method": "ping" }));
    assert_eq!(pong["result"], json!({}));

    // The tools are the readings of the contract, from the same list.
    let (_, listed) = rpc(&w, &w.reader, json!({ "jsonrpc": "2.0", "id": 4, "method": "tools/list" }));
    let tools = listed["result"]["tools"].as_array().unwrap();
    let names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
    let readings: Vec<&str> = offered_to(Caller::ProgramRead).map(|op| op.name).collect();
    assert_eq!(names, readings);
    assert!(!names.contains(&"caso_agregar_documentos"));
    assert!(tools.iter().all(|t| t["inputSchema"]["type"] == "object" && t["annotations"]["readOnlyHint"] == true));

    // A reading.
    let (_, found) = rpc(&w, &w.reader, json!({ "jsonrpc": "2.0", "id": 5, "method": "tools/call", "params": { "name": "buscar", "arguments": { "consulta": "carta documento" } } }));
    let result = &found["result"];
    assert_eq!(result["isError"], false, "{found}");
    assert!(result["structuredContent"]["resultados"].as_array().unwrap().iter().any(|r| r["nombre"] == "Carta documento 03-19.txt"), "{found}");
    assert!(result["content"][0]["text"].as_str().unwrap().contains("Carta documento"));
    let (_, matter) = rpc(&w, &w.reader, json!({ "jsonrpc": "2.0", "id": 6, "method": "tools/call", "params": { "name": "ver_caso", "arguments": { "caso": w.matter } } }));
    assert_eq!(matter["result"]["structuredContent"]["nombre"], "Prueba c/ Ejemplo SA");

    // A change from a program that only reads: an error of the tool, said in words.
    let change = json!({ "caso": w.matter, "documentos": [key(&w.base, "Carta documento 03-12.txt")] });
    let (_, refused) = rpc(&w, &w.reader, json!({ "jsonrpc": "2.0", "id": 7, "method": "tools/call", "params": { "name": "caso_agregar_documentos", "arguments": change } }));
    assert_eq!(refused["result"]["isError"], true);
    assert!(refused["result"]["content"][0]["text"].as_str().unwrap().contains("Leer y pedir cambios"));
    // What is not in the contract, or not MCP.
    let (_, unknown) = rpc(&w, &w.reader, json!({ "jsonrpc": "2.0", "id": 8, "method": "tools/call", "params": { "name": "borrar_todo", "arguments": {} } }));
    assert_eq!(unknown["error"]["code"], -32602);
    let (_, method) = rpc(&w, &w.reader, json!({ "jsonrpc": "2.0", "id": 9, "method": "resources/list" }));
    assert_eq!(method["error"]["code"], -32601);
    let (status, _) = w.server.post("/mcp", &bearer(&w.reader), "application/json", "{no es json");
    assert_eq!(status, 400);
    // No stream of events.
    let (status, _) = w.server.get("/mcp", &bearer(&w.reader));
    assert_eq!(status, 405);
}

#[test]
fn a_change_an_assistant_asks_for_by_mcp_waits_for_a_person() {
    let w = world();
    let (_, listed) = rpc(&w, &w.changer, json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/list" }));
    let tools = listed["result"]["tools"].as_array().unwrap();
    let add = tools.iter().find(|t| t["name"] == "caso_agregar_documentos").expect("the change is offered");
    assert!(add["description"].as_str().unwrap().contains("Atención"));
    assert_eq!(add["annotations"]["readOnlyHint"], false);
    assert!(!tools.iter().any(|t| t["name"] == "recorte_guardar" || t["name"] == "grupo_crear"));

    let change = json!({ "caso": "Prueba c/ Ejemplo SA", "documentos": [key(&w.base, "Carta documento 03-19.txt")] });
    let (_, asked) = rpc(&w, &w.changer, json!({ "jsonrpc": "2.0", "id": 2, "method": "tools/call", "params": { "name": "caso_agregar_documentos", "arguments": change } }));
    let answer = &asked["result"]["structuredContent"];
    assert_eq!(asked["result"]["isError"], false, "{asked}");
    assert_eq!(answer["estado"], "pendiente");
    let id = answer["pedido"].as_i64().unwrap();
    assert_eq!(matter_documents(&w.base, &w.matter), 1);
    assert_eq!(log::get(&w.base, id).unwrap().unwrap().caller, format!("program:{}", w.changer_id));
    assert_eq!(w.probe.proposed.load(Ordering::Relaxed), 1);

    // A person cancels it in the app: nothing is done, and the assistant learns it.
    flow::cancel(&w.base, id).unwrap();
    let (_, state) = rpc(&w, &w.changer, json!({ "jsonrpc": "2.0", "id": 3, "method": "tools/call", "params": { "name": "estado_pedido", "arguments": { "pedido": id } } }));
    assert_eq!(state["result"]["structuredContent"]["estado"], "cancelado", "{state}");
    assert_eq!(matter_documents(&w.base, &w.matter), 1);

    // Several messages at once.
    let (status, batch) = rpc(&w, &w.changer, json!([
        { "jsonrpc": "2.0", "id": 4, "method": "ping" },
        { "jsonrpc": "2.0", "method": "notifications/initialized" },
        { "jsonrpc": "2.0", "id": 5, "method": "tools/list" },
    ]));
    assert_eq!(status, 200);
    assert_eq!(batch.as_array().unwrap().len(), 2);
}
