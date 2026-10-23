mod agent;
mod clip_server;
mod commands;
mod docbase;
mod micelya_core;
mod panic_guard;
mod portals;
mod proxy;
mod secrets;
mod source_volume;
mod tray;
mod types;
mod work_folder;

use serde_json::Value;
use std::sync::Mutex;
use tauri::{Emitter, Manager};
use uuid::Uuid;

struct CloseBehaviorState(Mutex<String>);
struct TrayAvailabilityState(Mutex<bool>);

/// The folder the chat's agent works in (P-239): `<app data>/agent`, with
/// its sessions, its skills and the files it produces. It replaces the
/// folder of the open wiki project.
#[derive(Debug, Clone)]
struct AgentHome {
    id: String,
    path: String,
}

#[derive(Debug, Clone, Default)]
struct AgentRuntimeConfig {
    llm: Option<agent::provider::LlmConfig>,
    web_search: Option<agent::tools::WebSearchConfig>,
    anytxt: Option<agent::tools::AnyTxtConfig>,
}

#[tauri::command]
async fn agent_start_turn(
    app: tauri::AppHandle,
    mut request: agent::AgentChatRequest,
    llm_config: Option<agent::provider::LlmConfig>,
) -> Result<agent::types::AgentChatResponse, String> {
    let project = agent_home(&app)?;
    if request
        .session_id
        .as_deref()
        .map(str::trim)
        .unwrap_or("")
        .is_empty()
    {
        request.session_id = Some(format!("ui_{}", Uuid::new_v4()));
    }
    let active_session_id = request.session_id.clone().unwrap_or_default();
    if request
        .run_id
        .as_deref()
        .map(str::trim)
        .unwrap_or("")
        .is_empty()
    {
        request.run_id = Some(format!("run_{}", Uuid::new_v4()));
    }
    let active_run_id = request.run_id.clone().unwrap_or_default();
    if let Some(session_id) = request.session_id.clone() {
        if request.history.is_empty() && !request.history_explicit {
            request.history = app
                .state::<agent::session::AgentSessionStore>()
                .recent_messages(&project.path, &session_id, 12)
                .into_iter()
                .map(|message| agent::types::AgentConversationMessage {
                    role: message.role,
                    content: message.content,
                })
                .collect();
        }
    }
    let mut runtime_config = load_agent_runtime_config(&app);
    runtime_config.llm = llm_config.or(runtime_config.llm);
    let runtime = agent::AgentRuntime::new(
        project.id.clone(),
        project.path.clone(),
        runtime_config.llm,
        runtime_config.web_search,
        runtime_config.anytxt,
    )
    .with_base(docbase::bridge::shared_base());
    let user_message = request.message.clone();
    let persist_session = request.persist_session;
    let cancellation = app
        .state::<agent::cancel::AgentCancellationRegistry>()
        .start(&project.id, &active_session_id, &active_run_id);
    let result = runtime
        .run_once_with_cancel(request, Some(cancellation))
        .await;
    app.state::<agent::cancel::AgentCancellationRegistry>()
        .finish(&project.id, &active_session_id, &active_run_id);
    let response = result?;
    if persist_session {
        app.state::<agent::session::AgentSessionStore>()
            .append_turn(
                &project.path,
                &project.id,
                &response.session_id,
                &user_message,
                &response.message,
            );
    }
    Ok(response)
}

#[tauri::command]
fn agent_cancel_turn(
    app: tauri::AppHandle,
    session_id: String,
    run_id: Option<String>,
) -> Result<bool, String> {
    let project = agent_home(&app)?;
    Ok(app
        .state::<agent::cancel::AgentCancellationRegistry>()
        .cancel(&project.id, &session_id, run_id.as_deref()))
}

#[tauri::command]
async fn agent_start_turn_stream(
    app: tauri::AppHandle,
    mut request: agent::AgentChatRequest,
    llm_config: Option<agent::provider::LlmConfig>,
) -> Result<String, String> {
    let project = agent_home(&app)?;
    if request
        .session_id
        .as_deref()
        .map(str::trim)
        .unwrap_or("")
        .is_empty()
    {
        request.session_id = Some(format!("ui_{}", Uuid::new_v4()));
    }
    let active_session_id = request.session_id.clone().unwrap_or_default();
    if request
        .run_id
        .as_deref()
        .map(str::trim)
        .unwrap_or("")
        .is_empty()
    {
        request.run_id = Some(format!("run_{}", Uuid::new_v4()));
    }
    let active_run_id = request.run_id.clone().unwrap_or_default();
    if request.history.is_empty() && !request.history_explicit {
        request.history = app
            .state::<agent::session::AgentSessionStore>()
            .recent_messages(&project.path, &active_session_id, 12)
            .into_iter()
            .map(|message| agent::types::AgentConversationMessage {
                role: message.role,
                content: message.content,
            })
            .collect();
    }
    let mut runtime_config = load_agent_runtime_config(&app);
    runtime_config.llm = llm_config.or(runtime_config.llm);
    let runtime = agent::AgentRuntime::new(
        project.id.clone(),
        project.path.clone(),
        runtime_config.llm,
        runtime_config.web_search,
        runtime_config.anytxt,
    )
    .with_base(docbase::bridge::shared_base());
    let app_for_task = app.clone();
    let project_for_task = project.clone();
    let session_for_task = active_session_id.clone();
    let run_for_task = active_run_id.clone();
    let user_message = request.message.clone();
    let persist_session = request.persist_session;
    let cancellation = app
        .state::<agent::cancel::AgentCancellationRegistry>()
        .start(&project.id, &active_session_id, &active_run_id);
    tauri::async_runtime::spawn(async move {
        let emit_app = app_for_task.clone();
        let emit_session = session_for_task.clone();
        let emit_run = run_for_task.clone();
        let sink: agent::runtime::AgentEventSink = std::sync::Arc::new(move |event| {
            let _ = emit_app.emit(
                "agent-event",
                serde_json::json!({
                    "sessionId": emit_session.clone(),
                    "runId": emit_run.clone(),
                    "event": event,
                }),
            );
        });
        let result = runtime
            .run_once_with_cancel_and_events(request, Some(cancellation), Some(sink))
            .await;
        app_for_task
            .state::<agent::cancel::AgentCancellationRegistry>()
            .finish(&project_for_task.id, &session_for_task, &run_for_task);
        match result {
            Ok(response) => {
                if persist_session {
                    app_for_task
                        .state::<agent::session::AgentSessionStore>()
                        .append_turn(
                            &project_for_task.path,
                            &project_for_task.id,
                            &response.session_id,
                            &user_message,
                            &response.message,
                        );
                }
            }
            Err(err) => {
                let _ = app_for_task.emit(
                    "agent-event",
                    serde_json::json!({
                        "sessionId": session_for_task,
                        "runId": run_for_task,
                        "event": { "type": "error", "message": err },
                    }),
                );
            }
        }
    });
    Ok(active_session_id)
}

/// The agent's folder, so the chat can show and open what it produced and
/// the command-line models can work in it.
#[tauri::command]
fn agent_workspace(app: tauri::AppHandle) -> Result<String, String> {
    agent_home(&app).map(|home| home.path)
}

fn agent_home(app: &tauri::AppHandle) -> Result<AgentHome, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("no se encontró la carpeta de datos de la app: {e}"))?
        .join("agent");
    std::fs::create_dir_all(&dir).map_err(|e| format!("no se pudo crear '{}': {e}", dir.display()))?;
    Ok(AgentHome {
        id: "micelya".to_string(),
        path: dir.to_string_lossy().replace('\\', "/"),
    })
}

fn load_agent_app_state(app: &tauri::AppHandle) -> Option<Value> {
    let path = app.path().app_data_dir().ok()?.join("app-state.json");
    let raw = std::fs::read_to_string(path).ok()?;
    let mut parsed: Value = serde_json::from_str(&raw).ok()?;
    // The keys are in the keychain, not in the file (secrets.rs). Only the
    // main model and the web search are read here.
    if let Some(settings) = parsed.as_object_mut() {
        settings.remove("providerConfigs");
        secrets::fill_keys(settings, &secrets::Keychain);
    }
    Some(parsed)
}

fn load_agent_runtime_config(app: &tauri::AppHandle) -> AgentRuntimeConfig {
    let Some(parsed) = load_agent_app_state(app) else {
        return AgentRuntimeConfig::default();
    };
    AgentRuntimeConfig {
        llm: parsed
            .get("llmConfig")
            .cloned()
            .and_then(|value| serde_json::from_value(value).ok()),
        web_search: parsed
            .get("searchApiConfig")
            .cloned()
            .and_then(|value| serde_json::from_value(value).ok()),
        anytxt: parsed
            .get("searchApiConfig")
            .and_then(|value| value.get("anyTxt"))
            .cloned()
            .and_then(|value| serde_json::from_value(value).ok()),
    }
}

/// Apply a proxy configuration to the process env immediately, so the
/// next outbound HTTP request picks it up without needing the user to
/// restart the app. tauri-plugin-http builds a fresh
/// `reqwest::ClientBuilder` per fetch and reqwest's `auto_sys_proxy`
/// re-reads HTTP_PROXY / HTTPS_PROXY / NO_PROXY each time, so updating
/// these env vars is sufficient to flip the proxy on/off live.
///
/// Returns the same human-readable summary `apply_proxy_env` produces
/// for logging.
#[tauri::command]
fn set_proxy_env(config: proxy::ProxyConfig) -> String {
    let summary = proxy::apply_proxy_env(&config);
    eprintln!("[proxy] live update: {summary}");
    summary
}

#[tauri::command]
fn set_close_behavior(
    value: String,
    state: tauri::State<'_, CloseBehaviorState>,
) -> Result<String, String> {
    let normalized = match value.as_str() {
        "ask" | "minimize" | "exit" => value,
        other => return Err(format!("Invalid close behavior: {other}")),
    };
    let mut guard = state
        .0
        .lock()
        .map_err(|_| "Close behavior state is unavailable".to_string())?;
    *guard = normalized.clone();
    Ok(normalized)
}

fn close_behavior<R: tauri::Runtime>(window: &tauri::Window<R>) -> String {
    window
        .state::<CloseBehaviorState>()
        .0
        .lock()
        .map(|value| value.clone())
        .unwrap_or_else(|_| "minimize".to_string())
}

fn tray_available<R: tauri::Runtime>(window: &tauri::Window<R>) -> bool {
    window
        .state::<TrayAvailabilityState>()
        .0
        .lock()
        .map(|value| *value)
        .unwrap_or(false)
}

/// How the web clipper's local server is: `starting`, `running`,
/// `port-in-use` or `stopped`.
#[tauri::command]
fn clip_server_status() -> String {
    clip_server::state().to_string()
}

/// Every command of the app answers only to the app's own webview: the
/// view of a judicial portal, a remote page inside the same window, never
/// reaches them (D-447). Tauri already refuses them to a remote page that
/// no capability names; this does not depend on that.
fn only_app_webview<R: tauri::Runtime>(
    handler: impl Fn(tauri::ipc::Invoke<R>) -> bool + Send + Sync + 'static,
) -> impl Fn(tauri::ipc::Invoke<R>) -> bool + Send + Sync + 'static {
    move |invoke| {
        if !portals::is_app_webview(invoke.message.webview_ref().label()) {
            invoke.resolver.reject("Este pedido solo se acepta desde la ventana de la app");
            return true;
        }
        handler(invoke)
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    apply_linux_webkit_compat_env();

    tauri::Builder::default()
        // Has to be the first plugin. Closing the window only hides the
        // app in the tray, so opening it again used to start a second
        // copy that fought the first one for the local ports. A second
        // launch now brings back the window of the one already running.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            tray::show_main_window(app);
        }))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None::<Vec<&str>>,
        ))
        // Rust-backed fetch so third-party LLM APIs that reject
        // browser-origin headers via CORS preflight (MiniMax, Volcengine
        // Ark's api/coding/v3, etc.) still work. Requests leave the app
        // from Rust, never the webview.
        .plugin(tauri_plugin_http::init())
        .setup(|app| {
            // Let the PDF extractor find the bundled pdfium dynamic
            // library via Tauri's platform-correct resource path.
            if let Ok(dir) = app.path().resource_dir() {
                commands::fs::set_resource_dir_hint(dir);
            }
            // Apply user-configured global HTTP proxy by setting
            // HTTP_PROXY / HTTPS_PROXY / NO_PROXY env vars BEFORE
            // any HTTP request is made. tauri-plugin-http's reqwest
            // client reads these on first construction. Lives next
            // to the resource-dir hint so the proxy applies to
            // everything: LLM, embedding, update check, deep
            // research, captioning. See src-tauri/src/proxy.rs.
            if let Ok(dir) = app.path().app_data_dir() {
                let store_path = dir.join("app-state.json");
                eprintln!("[proxy] reading from {}", store_path.display());
                if let Some(cfg) = proxy::read_proxy_config_from_store(&store_path) {
                    let summary = proxy::apply_proxy_env(&cfg);
                    eprintln!("[proxy] {summary}");
                } else {
                    eprintln!("[proxy] no proxyConfig in store, requests go direct");
                }
            } else {
                eprintln!("[proxy] could not resolve app_data_dir");
            }
            // The readings this computer asked for in Micelya Core and not
            // deleted yet are kept beside the app's data (P-294).
            if let Ok(dir) = app.path().app_data_dir() {
                micelya_core::ledger::install(&dir);
            }
            // The API keys still written in app-state.json move to the
            // system keychain, before the screens read the settings
            // (secrets.rs, D-444).
            secrets::move_keys_of_app_state(app.handle());
            // Registry of running `claude` subprocesses, keyed by the
            // frontend-generated stream id. Populated by claude_cli_spawn,
            // drained on process exit or by claude_cli_kill.
            app.manage(commands::claude_cli::ClaudeCliState::default());
            app.manage(commands::codex_cli::CodexCliState::default());
            app.manage(agent::session::AgentSessionStore::default());
            app.manage(agent::cancel::AgentCancellationRegistry::default());
            app.manage(CloseBehaviorState(Mutex::new("minimize".to_string())));
            app.manage(TrayAvailabilityState(Mutex::new(false)));
            app.manage(portals::view::PortalState::default());
            docbase::commands::start(app.handle());
            // The work folder exists with its folders and is an origin
            // (D-450); the web clipper's server saves the clips there.
            work_folder::start(app.handle());
            clip_server::start(app.handle().clone());
            let tray_available = match tray::create_tray(app.handle()) {
                Ok(()) => true,
                Err(err) => {
                    eprintln!("[tray] system tray unavailable, continuing without it: {err}");
                    false
                }
            };
            match app.state::<TrayAvailabilityState>().0.lock() {
                Ok(mut state) => {
                    *state = tray_available;
                }
                Err(err) => {
                    eprintln!("[tray] failed to update tray availability state: {err}");
                }
            }
            Ok(())
        })
        // A reload of the app's page leaves the portals' views on top of
        // the new page: they are hidden until «Portales» asks for them.
        .on_page_load(portals::view::hide_on_app_reload)
        .invoke_handler(only_app_webview::<tauri::Wry>(tauri::generate_handler![
            commands::fs::read_file,
            commands::fs::write_file,
            commands::fs::list_directory,
            docbase::commands::docbase_list_origins,
            docbase::commands::docbase_preview_folder,
            docbase::commands::docbase_add_origin,
            docbase::commands::docbase_remove_origin,
            docbase::commands::docbase_rescan,
            docbase::commands::docbase_home,
            docbase::commands::docbase_list_documents,
            docbase::commands::docbase_list_folder,
            docbase::commands::docbase_set_excluded,
            docbase::commands::docbase_open_folder,
            docbase::commands::docbase_get_document,
            docbase::commands::docbase_document_copies,
            docbase::commands::docbase_label_groups,
            docbase::commands::docbase_ensure_label,
            docbase::commands::docbase_rename_label,
            docbase::commands::docbase_delete_label,
            docbase::commands::docbase_rename_group,
            docbase::commands::docbase_delete_group,
            docbase::commands::docbase_label_rules,
            docbase::commands::docbase_add_fixed_rule,
            docbase::commands::docbase_set_levels_rule,
            docbase::commands::docbase_remove_rule,
            docbase::commands::docbase_set_location_label,
            docbase::commands::docbase_open_location,
            docbase::commands::docbase_similar_documents,
            docbase::commands::docbase_decide_similar,
            docbase::commands::docbase_similar_pending,
            docbase::commands::docbase_decide_similar_many,
            docbase::commands::docbase_similar_open_pairs,
            docbase::commands::docbase_similar_groups,
            docbase::commands::docbase_similar_relations,
            docbase::commands::docbase_similar_template,
            docbase::commands::docbase_templates,
            docbase::commands::docbase_name_template,
            docbase::commands::docbase_choose_template_field,
            docbase::commands::docbase_generate_document,
            docbase::commands::docbase_open_generated,
            docbase::commands::docbase_types_plan,
            docbase::commands::docbase_proposed_types,
            docbase::commands::docbase_decide_type,
            docbase::commands::docbase_decided_types,
            docbase::commands::docbase_types_run,
            docbase::commands::docbase_ai_services,
            docbase::commands::docbase_set_ai_services,
            docbase::commands::docbase_model_questions,
            docbase::commands::docbase_model_question_last,
            docbase::commands::docbase_document_type,
            docbase::commands::docbase_document_types,
            docbase::commands::docbase_knowledge_status,
            docbase::commands::docbase_graph,
            docbase::commands::docbase_knowledge_plan,
            docbase::commands::docbase_knowledge_document,
            docbase::commands::docbase_knowledge_run,
            docbase::commands::docbase_knowledge_project,
            docbase::commands::docbase_set_wiki_language,
            docbase::commands::docbase_knowledge_resolve,
            docbase::commands::docbase_knowledge_entities,
            docbase::commands::docbase_knowledge_entity,
            docbase::commands::docbase_knowledge_article,
            docbase::commands::docbase_knowledge_document_article,
            docbase::commands::docbase_knowledge_object_of_page,
            docbase::commands::docbase_knowledge_notes,
            docbase::commands::docbase_knowledge_set_notes,
            docbase::commands::docbase_knowledge_orphan_notes,
            docbase::commands::docbase_knowledge_delete_note,
            docbase::commands::docbase_knowledge_overview,
            docbase::commands::docbase_knowledge_page_words,
            docbase::commands::docbase_search,
            docbase::commands::docbase_saved_queries,
            docbase::commands::docbase_knowledge_page_text,
            docbase::commands::docbase_research_save,
            docbase::commands::docbase_research_of,
            docbase::commands::docbase_research_loose,
            docbase::commands::docbase_research_delete,
            docbase::commands::docbase_chat_folder,
            docbase::commands::docbase_data_status,
            docbase::commands::docbase_open_data_folder,
            docbase::commands::docbase_restore_backup,
            docbase::commands::docbase_close_app,
            docbase::commands::docbase_save_query,
            docbase::commands::docbase_delete_saved_query,
            docbase::commands::docbase_background_reading,
            docbase::commands::docbase_set_background_reading,
            docbase::commands::docbase_progress,
            docbase::commands::docbase_recognize_text,
            docbase::commands::docbase_progress_items,
            docbase::commands::docbase_progress_folders,
            docbase::commands::docbase_knowledge_doubts,
            docbase::commands::docbase_knowledge_decisions,
            docbase::commands::docbase_knowledge_decide,
            docbase::commands::docbase_knowledge_undo,
            docbase::commands::docbase_knowledge_read_again_failed,
            docbase::commands::docbase_knowledge_reread_plan,
            docbase::commands::docbase_knowledge_reread,
            docbase::commands::docbase_auto_questions_decide,
            docbase::commands::docbase_knowledge_decide_many,
            docbase::commands::docbase_knowledge_attention,
            docbase::commands::docbase_knowledge_matters,
            docbase::commands::docbase_knowledge_matter,
            docbase::commands::docbase_knowledge_document_entities,
            docbase::commands::docbase_knowledge_leave_matter,
            docbase::commands::docbase_knowledge_join_matter,
            docbase::commands::docbase_knowledge_decide_matter,
            docbase::commands::docbase_knowledge_create_matter,
            docbase::commands::docbase_knowledge_rename_matter,
            docbase::commands::docbase_knowledge_choose_matter_document,
            commands::fs::delete_file,
            commands::fs::create_directory,
            commands::fs::get_file_size,
            commands::fs::read_file_as_base64,
            commands::external_search::web_search,
            commands::external_search::anytxt_search,
            agent_start_turn,
            agent_start_turn_stream,
            agent_cancel_turn,
            agent_workspace,
            agent::skills::agent_list_skills,
            commands::claude_cli::claude_cli_detect,
            commands::claude_cli::claude_cli_spawn,
            commands::claude_cli::claude_cli_kill,
            commands::codex_cli::codex_cli_detect,
            commands::codex_cli::codex_cli_spawn,
            commands::codex_cli::codex_cli_kill,
            set_proxy_env,
            set_close_behavior,
            portals::view::portal_show,
            portals::view::portal_hide,
            portals::view::portal_fetch,
            portals::view::portal_update_all,
            portals::view::portal_stop,
            portals::view::portal_go_on,
            portals::view::portal_manual,
            portals::view::portal_status,
            portals::view::portal_followed,
            portals::view::portal_jurisdictions,
            portals::view::portal_open_folder,
            work_folder::work_folder_status,
            secrets::secrets_read,
            secrets::secrets_write,
            micelya_core::micelya_core_connection,
            micelya_core::micelya_core_save_connection,
            micelya_core::micelya_core_remove_connection,
            micelya_core::micelya_core_test_connection,
            micelya_core::micelya_core_left_readings,
            micelya_core::micelya_core_delete_left_readings,
            work_folder::work_folder_set,
            work_folder::work_folder_open,
            work_folder::work_folder_new_clips,
            work_folder::work_folder_clips_seen,
            clip_server_status,
        ]))
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let behavior = close_behavior(window);
                let win = window.clone();
                let app = window.app_handle().clone();
                match behavior.as_str() {
                    "exit" => {
                        tauri::async_runtime::spawn(async move {
                            let _ = win.destroy();
                            app.exit(0);
                        });
                    }
                    "minimize" => {
                        if tray_available(window) {
                            let _ = window.hide();
                        } else {
                            let _ = window.minimize();
                        }
                    }
                    _ => {
                        tauri::async_runtime::spawn(async move {
                            use tauri_plugin_dialog::{DialogExt, MessageDialogButtons};
                            let confirmed = app
                                .dialog()
                                .message(
                                    "Quit Micelya Desktop? Choose Quit to exit. Choose Hide Window to keep background features running.",
                                )
                                .title("Micelya Desktop")
                                .buttons(MessageDialogButtons::OkCancelCustom(
                                    "Quit".to_string(),
                                    "Hide Window".to_string(),
                                ))
                                .kind(tauri_plugin_dialog::MessageDialogKind::Warning)
                                .blocking_show();

                            if confirmed {
                                let _ = win.destroy();
                                app.exit(0);
                            } else {
                                let _ = win.hide();
                            }
                        });
                    }
                }
            }
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| {
            #[cfg(target_os = "macos")]
            if let tauri::RunEvent::Reopen {
                has_visible_windows,
                ..
            } = event
            {
                if !has_visible_windows {
                    use tauri::Manager;
                    // `get_window`: with the portal's view inside it, the
                    // main window is no longer a single-webview window.
                    if let Some(window) = app.get_window("main") {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                }
            }
            let _ = (app, event); // suppress unused warnings on non-macOS
        });
}

#[cfg(target_os = "linux")]
fn apply_linux_webkit_compat_env() {
    // WebKitGTK can crash or withdraw its window on some Wayland/XWayland
    // stacks unless accelerated render paths are disabled before the WebView
    // is created. Keep these as Linux-only defaults and do not override an
    // explicit user setting so advanced users and packagers can opt back into
    // the platform default if their stack supports it.
    if std::env::var_os("WEBKIT_DISABLE_COMPOSITING_MODE").is_none() {
        std::env::set_var("WEBKIT_DISABLE_COMPOSITING_MODE", "1");
    }
    // Some WebKitGTK/Mesa combinations still attempt the DMA-BUF renderer
    // even with accelerated compositing disabled. In an AppImage running
    // through XWayland that can withdraw the native window while leaving the
    // web and network processes alive. Respect explicit packager overrides.
    if std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none() {
        std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
    }
}

#[cfg(not(target_os = "linux"))]
fn apply_linux_webkit_compat_env() {}
