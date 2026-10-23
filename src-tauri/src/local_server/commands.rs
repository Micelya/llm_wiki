//! The commands of Ajustes for the server inside the machine. Like every
//! command, only the app's own window reaches them (D-447).

use serde::Serialize;

/// How the server is, for Ajustes.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerStatus {
    /// `starting`, `running`, `port-in-use` or `stopped`.
    pub state: String,
    /// `http://127.0.0.1:19827`.
    pub address: String,
}

#[tauri::command]
pub fn local_server_status() -> ServerStatus {
    ServerStatus { state: super::state().to_string(), address: super::address() }
}
