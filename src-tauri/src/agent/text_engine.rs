//! Codex as an engine of only text for the chat's agent (3.3, 11.6): the
//! same agent as the engines with an API, so Codex sees the instructions
//! in Spanish, takes turns to search and read, and can propose a change of
//! the contract (P-288). Each turn is one run of `codex exec` in read-only
//! mode, ephemeral, with approvals off, in the agent's folder, with the
//! whole request in one text with its parts marked. Codex gets no tool of
//! the base and no permission to change anything: it only writes the next
//! step, and the program does it.
//!
//! Cancelling the turn kills the process: the child is killed when the
//! future that waits for it is dropped.

use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::process::Stdio;
use std::time::Duration;

use tokio::io::AsyncWriteExt;

use super::provider::{AgentLlmProvider, LlmConfig};
use super::types::AgentImage;

const DEFAULT_TIMEOUT_MINUTES: u64 = 10;
const STDERR_TAIL_CHARS: usize = 2_000;

/// The reasoning levels the app offers for Codex (D-274), and the one used
/// when none was chosen (D-279).
const LEVELS: &[&str] = &["low", "medium", "high"];
const DEFAULT_LEVEL: &str = "high";

#[derive(Debug, Clone)]
pub struct CodexTextEngine {
    model: String,
    reasoning: String,
    isolate_local_config: bool,
    timeout: Duration,
    working_dir: PathBuf,
    /// What the record says proposed a change: `codex-cli:<model>@<level>`.
    label: String,
}

impl CodexTextEngine {
    pub fn from_config(config: &LlmConfig, working_dir: &str) -> Self {
        let model = config.model.trim().to_string();
        let asked = config.codex_reasoning_effort.as_deref().unwrap_or_default().trim().to_lowercase();
        let reasoning = if LEVELS.contains(&asked.as_str()) { asked } else { DEFAULT_LEVEL.to_string() };
        let minutes = config.codex_cli_timeout_minutes.unwrap_or(DEFAULT_TIMEOUT_MINUTES).clamp(1, 240);
        let label = if model.is_empty() { format!("codex-cli@{reasoning}") } else { format!("codex-cli:{model}@{reasoning}") };
        Self {
            model,
            reasoning,
            isolate_local_config: config.local_cli_isolation.unwrap_or(false),
            timeout: Duration::from_secs(minutes * 60),
            working_dir: PathBuf::from(working_dir),
            label,
        }
    }

    /// The arguments of one turn: read-only, ephemeral, approvals off, the
    /// answer written to `answer_file`, the request from stdin.
    pub fn args(&self, answer_file: &Path) -> Vec<String> {
        let mut args = vec!["-a".to_string(), "never".to_string(), "exec".to_string()];
        if self.isolate_local_config {
            args.extend(["--ignore-user-config".to_string(), "--ignore-rules".to_string()]);
        }
        args.extend([
            "--skip-git-repo-check".to_string(),
            "--sandbox".to_string(),
            "read-only".to_string(),
            "--ephemeral".to_string(),
        ]);
        if !self.model.is_empty() {
            args.extend(["--model".to_string(), self.model.clone()]);
        }
        args.extend(["-c".to_string(), format!("model_reasoning_effort={}", self.reasoning)]);
        args.extend(["-o".to_string(), answer_file.to_string_lossy().to_string(), "-".to_string()]);
        args
    }

    async fn run(&self, prompt: String) -> Result<String, String> {
        let codex = crate::commands::cli_resolver::find_cli_command("codex", &["codex.cmd", "codex.exe"])
            .await
            .map_err(|_| "No se encontró Codex en esta computadora (`codex`): instalalo o elegí otro motor para el chat.".to_string())?;
        let scratch = std::env::temp_dir().join(format!("micelya-chat-codex-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&scratch).map_err(|e| format!("No se pudo crear '{}': {e}", scratch.display()))?;
        let answer_file = scratch.join("respuesta.txt");
        let result = self.run_in(&codex, &answer_file, prompt).await;
        let _ = std::fs::remove_dir_all(&scratch);
        result
    }

    async fn run_in(&self, codex: &Path, answer_file: &Path, prompt: String) -> Result<String, String> {
        let mut cmd = tokio::process::Command::new(codex);
        #[cfg(windows)]
        {
            const CREATE_NO_WINDOW: u32 = 0x08000000;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }
        if let Some(path_env) = crate::commands::cli_resolver::child_path_env().await {
            cmd.env("PATH", path_env);
        }
        let working_dir = if self.working_dir.is_dir() { self.working_dir.clone() } else { std::env::temp_dir() };
        cmd.args(self.args(answer_file))
            .current_dir(working_dir)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        let mut child = cmd.spawn().map_err(|e| format!("No se pudo iniciar Codex: {e}"))?;
        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(prompt.as_bytes()).await.map_err(|e| format!("No se pudo mandar el pedido a Codex: {e}"))?;
            stdin.flush().await.map_err(|e| format!("No se pudo mandar el pedido a Codex: {e}"))?;
        }
        let output = match tokio::time::timeout(self.timeout, child.wait_with_output()).await {
            Ok(Ok(output)) => output,
            Ok(Err(e)) => return Err(format!("No se pudo esperar a Codex: {e}")),
            Err(_) => return Err(format!("Codex no contestó en {} minutos", self.timeout.as_secs() / 60)),
        };
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let tail: String = stderr.chars().rev().take(STDERR_TAIL_CHARS).collect::<Vec<_>>().into_iter().rev().collect();
            return Err(format!("Codex terminó con {}: {}", output.status, tail.trim()));
        }
        let text = std::fs::read_to_string(answer_file).map_err(|e| format!("Codex no escribió una respuesta: {e}"))?;
        if text.trim().is_empty() {
            return Err("Codex devolvió una respuesta vacía".to_string());
        }
        Ok(text)
    }
}

/// The whole request in one text: the instructions, then the parts the
/// agent marked (the conversation, the documents, what was observed, the
/// request).
pub fn one_text(system: &str, user: &str) -> String {
    format!("<INSTRUCCIONES>\n{}\n</INSTRUCCIONES>\n\n{}", system.trim(), user.trim())
}

impl AgentLlmProvider for CodexTextEngine {
    fn provider_name(&self) -> &str {
        "codex-cli"
    }

    fn model_name(&self) -> &str {
        &self.label
    }

    fn generate_text<'a>(
        &'a self,
        system: &'a str,
        user: &'a str,
        _images: &'a [AgentImage],
    ) -> Pin<Box<dyn Future<Output = Result<String, String>> + Send + 'a>> {
        Box::pin(async move { self.run(one_text(system, user)).await })
    }

    fn generate_text_stream<'a>(
        &'a self,
        system: &'a str,
        user: &'a str,
        images: &'a [AgentImage],
        mut on_delta: Box<dyn FnMut(&str) + Send + 'a>,
    ) -> Pin<Box<dyn Future<Output = Result<String, String>> + Send + 'a>> {
        Box::pin(async move {
            let text = self.generate_text(system, user, images).await?;
            on_delta(&text);
            Ok(text)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config(json: serde_json::Value) -> LlmConfig {
        serde_json::from_value(json).unwrap()
    }

    #[test]
    fn a_turn_of_codex_is_read_only_ephemeral_without_approvals_and_comes_from_stdin() {
        let engine = CodexTextEngine::from_config(
            &config(serde_json::json!({ "provider": "codex-cli", "model": "gpt-6-luna", "codexReasoningEffort": "medium" })),
            "C:/agente",
        );
        let args = engine.args(Path::new("respuesta.txt"));
        assert!(args.windows(2).any(|w| w == ["-a", "never"]));
        assert!(args.windows(2).any(|w| w == ["--sandbox", "read-only"]));
        assert!(args.contains(&"--ephemeral".to_string()));
        assert!(args.windows(2).any(|w| w == ["--model", "gpt-6-luna"]));
        assert!(args.windows(2).any(|w| w == ["-c", "model_reasoning_effort=medium"]));
        assert!(args.windows(2).any(|w| w == ["-o", "respuesta.txt"]));
        assert_eq!(args.last().unwrap(), "-");
        assert!(!args.contains(&"--ignore-user-config".to_string()));
        assert!(!args.iter().any(|a| a == "--json" || a.contains("workspace-write") || a.contains("danger")));
        assert_eq!(engine.model_name(), "codex-cli:gpt-6-luna@medium");
    }

    #[test]
    fn without_a_level_codex_reasons_high_and_isolation_ignores_the_local_config() {
        let engine = CodexTextEngine::from_config(
            &config(serde_json::json!({ "provider": "codex-cli", "model": "", "localCliIsolation": true, "codexCliTimeoutMinutes": 0 })),
            "C:/agente",
        );
        let args = engine.args(Path::new("r.txt"));
        assert!(args.windows(2).any(|w| w == ["-c", "model_reasoning_effort=high"]));
        assert!(!args.contains(&"--model".to_string()));
        assert!(args.contains(&"--ignore-user-config".to_string()) && args.contains(&"--ignore-rules".to_string()));
        assert_eq!(engine.timeout, Duration::from_secs(60));
    }

    #[test]
    fn the_request_is_one_text_with_its_parts_marked() {
        let text = one_text("Sos el asistente.", "<PEDIDO>\nhola\n</PEDIDO>");
        assert!(text.starts_with("<INSTRUCCIONES>\nSos el asistente.\n</INSTRUCCIONES>"));
        assert!(text.ends_with("<PEDIDO>\nhola\n</PEDIDO>"));
    }
}
