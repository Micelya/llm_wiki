//! The commands of Ajustes → «Programas conectados» for the server inside
//! the machine. Like every command, only the app's own window reaches them
//! (D-447): a program never adds itself nor reads a token.

use serde::Serialize;
use tauri::State;

use crate::docbase::commands::DocBaseState;
use crate::secrets::Keychain;

use super::programs::{self, Permission, Program, WithToken};

/// How the server is, for Ajustes.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerStatus {
    /// `starting`, `running`, `port-in-use` or `stopped`.
    pub state: String,
    /// `http://127.0.0.1:19827`.
    pub address: String,
    /// Where an assistant connects by MCP.
    pub mcp_address: String,
    /// Whether programs may use it (the clipper always may).
    pub programs_enabled: bool,
}

#[tauri::command]
pub fn local_server_status() -> ServerStatus {
    ServerStatus {
        state: super::state().to_string(),
        address: super::address(),
        mcp_address: format!("{}/mcp", super::address()),
        programs_enabled: super::programs_on(),
    }
}

/// Turns the programs on or off (the web clipper keeps working).
#[tauri::command(async)]
pub fn local_server_set_enabled(state: State<'_, DocBaseState>, enabled: bool) -> Result<ServerStatus, String> {
    programs::set_enabled(&*state.shared()?, enabled)?;
    super::set_programs_on(enabled);
    Ok(local_server_status())
}

#[tauri::command(async)]
pub fn local_programs(state: State<'_, DocBaseState>) -> Result<Vec<Program>, String> {
    programs::list(&*state.shared()?)
}

/// Adds a program; gives its token, to copy into it.
#[tauri::command(async)]
pub fn local_program_add(state: State<'_, DocBaseState>, name: String, permission: String) -> Result<WithToken, String> {
    programs::add(&*state.shared()?, &Keychain, &name, Permission::parse(&permission)?)
}

/// The token of a program, to copy it again.
#[tauri::command(async)]
pub fn local_program_token(state: State<'_, DocBaseState>, id: String) -> Result<String, String> {
    programs::token(&*state.shared()?, &Keychain, &id)
}

/// A new token for a program: the old one stops working.
#[tauri::command(async)]
pub fn local_program_renew(state: State<'_, DocBaseState>, id: String) -> Result<WithToken, String> {
    programs::renew(&*state.shared()?, &Keychain, &id)
}

#[tauri::command(async)]
pub fn local_program_set_permission(state: State<'_, DocBaseState>, id: String, permission: String) -> Result<Program, String> {
    programs::set_permission(&*state.shared()?, &id, Permission::parse(&permission)?)
}

#[tauri::command(async)]
pub fn local_program_rename(state: State<'_, DocBaseState>, id: String, name: String) -> Result<Program, String> {
    programs::rename(&*state.shared()?, &id, &name)
}

/// Removes a program: its token stops working.
#[tauri::command(async)]
pub fn local_program_remove(state: State<'_, DocBaseState>, id: String) -> Result<(), String> {
    programs::remove(&*state.shared()?, &Keychain, &id)
}
