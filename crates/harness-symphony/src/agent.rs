use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use serde_json::{json, Value};
use thiserror::Error;

use crate::config::ResolvedConfig;
use crate::harness_protocol::{resolve_executable, HarnessProtocolError};
use crate::run::PreparedRun;

#[cfg(not(test))]
const CODEX_IDLE_RECONCILE_SECONDS: u64 = 30;
#[cfg(test)]
const CODEX_IDLE_RECONCILE_SECONDS: u64 = 1;

#[derive(Debug, Error)]
pub enum AgentError {
    #[error("agent.command is not configured. Set agent.command in .harness/symphony.yml.")]
    MissingCommand,
    #[error("selected agent executable '{0}' is not available; install it or configure agent.command before launching a run")]
    UnavailableCommand(String),
    #[error("unsupported agent adapter '{0}'. Supported adapters: custom, codex")]
    UnsupportedAdapter(String),
    #[error("agent command failed with status {status}: {stderr}")]
    CommandFailed { status: String, stderr: String },
    #[error("codex app-server failed: {0}")]
    Codex(String),
    #[error("agent io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("agent json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("{0}")]
    Protocol(#[from] HarnessProtocolError),
}

pub fn run_agent(config: &ResolvedConfig, prepared: &PreparedRun) -> Result<(), AgentError> {
    match config.agent_adapter.as_str() {
        "custom" => run_custom_agent(config, prepared),
        "codex" => run_codex_agent(config, prepared),
        other => Err(AgentError::UnsupportedAdapter(other.to_owned())),
    }
}

pub fn resolved_agent_command(config: &ResolvedConfig) -> Vec<String> {
    if !config.agent_command.is_empty() {
        return config.agent_command.clone();
    }
    if config.agent_adapter == "codex" {
        return vec!["codex".to_owned(), "app-server".to_owned()];
    }
    Vec::new()
}

pub fn agent_adapter_status(config: &ResolvedConfig) -> Result<String, AgentError> {
    match config.agent_adapter.as_str() {
        "custom" => {
            let command = resolved_agent_command(config);
            validate_agent_command(config, &command)?;
            Ok(format!("custom command: {}", command.join(" ")))
        }
        "codex" => {
            let command = resolved_agent_command(config);
            validate_agent_command(config, &command)?;
            Ok(format!(
                "codex app-server command: {}; runtime: uncapped",
                command.join(" ")
            ))
        }
        other => Err(AgentError::UnsupportedAdapter(other.to_owned())),
    }
}

/// Validate the selected execution adapter before run preparation creates a
/// branch, worktree, snapshot, or local state row. Prepare-only deliberately
/// does not call this function because it does not launch an agent.
pub fn preflight_agent(config: &ResolvedConfig) -> Result<(), AgentError> {
    agent_adapter_status(config).map(|_| ())
}

fn validate_agent_command(config: &ResolvedConfig, command: &[String]) -> Result<(), AgentError> {
    let Some(executable) = command.first() else {
        return Err(AgentError::MissingCommand);
    };
    if executable_available(executable, &config.repo_root) {
        Ok(())
    } else {
        Err(AgentError::UnavailableCommand(executable.clone()))
    }
}

fn executable_available(executable: &str, repo_root: &Path) -> bool {
    let path = Path::new(executable);
    if path.is_absolute() {
        return is_executable_file(path);
    }
    if path.components().count() > 1 {
        return is_executable_file(&repo_root.join(path));
    }
    let Some(search_path) = std::env::var_os("PATH") else {
        return false;
    };
    executable_candidates(executable).iter().any(|name| {
        std::env::split_paths(&search_path)
            .any(|directory| is_executable_file(&directory.join(name)))
    })
}

fn is_executable_file(path: &Path) -> bool {
    let Ok(metadata) = path.metadata() else {
        return false;
    };
    if !metadata.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        metadata.permissions().mode() & 0o111 != 0
    }
    #[cfg(windows)]
    {
        true
    }
}

fn executable_candidates(executable: &str) -> Vec<PathBuf> {
    #[cfg(not(windows))]
    let candidates = vec![PathBuf::from(executable)];
    #[cfg(windows)]
    let mut candidates = vec![PathBuf::from(executable)];
    #[cfg(windows)]
    if Path::new(executable).extension().is_none() {
        let extensions = std::env::var_os("PATHEXT")
            .and_then(|value| value.into_string().ok())
            .unwrap_or_else(|| ".COM;.EXE;.BAT;.CMD".to_owned());
        candidates.extend(
            extensions
                .split(';')
                .filter(|extension| !extension.is_empty())
                .map(|extension| PathBuf::from(format!("{executable}{extension}"))),
        );
    }
    candidates
}

fn run_custom_agent(config: &ResolvedConfig, prepared: &PreparedRun) -> Result<(), AgentError> {
    let command = resolved_agent_command(config);
    if command.is_empty() {
        return Err(AgentError::MissingCommand);
    }
    let output = base_command(&command, prepared).output()?;
    if output.status.success() {
        return Ok(());
    }
    Err(AgentError::CommandFailed {
        status: output.status.to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).trim().to_owned(),
    })
}

fn run_codex_agent(config: &ResolvedConfig, prepared: &PreparedRun) -> Result<(), AgentError> {
    let command = resolved_agent_command(config);
    if command.is_empty() {
        return Err(AgentError::MissingCommand);
    }

    let mut child = base_command(&command, prepared)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| AgentError::Codex("failed to open app-server stdin".to_owned()))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| AgentError::Codex("failed to open app-server stdout".to_owned()))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| AgentError::Codex("failed to open app-server stderr".to_owned()))?;

    let (line_tx, line_rx) = mpsc::channel::<String>();
    std::thread::spawn(move || {
        let reader = BufReader::new(stdout);
        for line in reader.lines().map_while(Result::ok) {
            if line_tx.send(line).is_err() {
                break;
            }
        }
    });

    send(
        &mut stdin,
        json!({
            "method": "initialize",
            "id": 0,
            "params": {
                "clientInfo": {
                    "name": "harness_symphony",
                    "title": "Harness Symphony",
                    "version": env!("CARGO_PKG_VERSION")
                },
                "capabilities": {
                    "experimentalApi": true,
                    "requestAttestation": false
                }
            }
        }),
    )?;

    let event_log_path = prepared
        .contract_path
        .parent()
        .unwrap_or(&prepared.worktree)
        .join("APP_SERVER_EVENTS.jsonl");
    let mut thread_id: Option<String> = None;
    let mut turn_id: Option<String> = None;
    let mut turn_started = false;
    let mut last_event_at = Instant::now();
    let mut last_observed_method = "none".to_owned();
    let mut event_count: u64 = 0;
    let mut next_request_id: i64 = 3;
    let mut pending_state_query: Option<i64> = None;
    loop {
        let line = match line_rx.recv_timeout(Duration::from_millis(250)) {
            Ok(line) => line,
            Err(mpsc::RecvTimeoutError::Timeout) => {
                if let Some(status) = child.try_wait()? {
                    let stderr = read_child_stderr(stderr)?;
                    return Err(AgentError::CommandFailed {
                        status: status.to_string(),
                        stderr,
                    });
                }
                if pending_state_query.is_some()
                    && last_event_at.elapsed() >= Duration::from_secs(CODEX_IDLE_RECONCILE_SECONDS)
                {
                    terminate_child(&mut child);
                    return Err(AgentError::Codex(format!(
                        "no app-server events or turn-state response for {} second(s) after reconciliation request. Last app-server method: {last_observed_method}; events: {event_count}; see {}",
                        CODEX_IDLE_RECONCILE_SECONDS,
                        event_log_path.display()
                    )));
                }
                if let (Some(thread_id), Some(_turn_id)) = (&thread_id, &turn_id) {
                    if pending_state_query.is_none()
                        && turn_started
                        && last_event_at.elapsed()
                            >= Duration::from_secs(CODEX_IDLE_RECONCILE_SECONDS)
                    {
                        let request_id = next_request_id;
                        next_request_id += 1;
                        send_turn_state_query(&mut stdin, request_id, thread_id)?;
                        pending_state_query = Some(request_id);
                        last_event_at = Instant::now();
                    }
                }
                continue;
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                let status = child.wait()?;
                let stderr = read_child_stderr(stderr)?;
                return Err(AgentError::CommandFailed {
                    status: status.to_string(),
                    stderr,
                });
            }
        };

        append_event_log(&event_log_path, &line)?;
        let message: Value = serde_json::from_str(&line)?;
        event_count += 1;
        last_event_at = Instant::now();
        if let Some(method) = message.get("method").and_then(Value::as_str) {
            last_observed_method = method.to_owned();
        } else if let Some(id) = message.get("id").and_then(Value::as_i64) {
            last_observed_method = format!("response:{id}");
        }
        if let Some(error) = message.get("error") {
            if pending_state_query == message.get("id").and_then(Value::as_i64) {
                terminate_child(&mut child);
                return Err(AgentError::Codex(format!(
                    "turn-state query failed: {error}. Last app-server method: {last_observed_method}; events: {event_count}; see {}",
                    event_log_path.display()
                )));
            }
            terminate_child(&mut child);
            return Err(AgentError::Codex(error.to_string()));
        }

        if message.get("id").is_some() && message.get("method").is_some() {
            let method = message
                .get("method")
                .and_then(Value::as_str)
                .unwrap_or("unknown");
            terminate_child(&mut child);
            return Err(AgentError::Codex(format!(
                "unsupported app-server request '{method}'. See {}",
                event_log_path.display()
            )));
        }

        let response_id = message.get("id").and_then(Value::as_i64);
        match response_id {
            Some(0) => {
                send(&mut stdin, json!({ "method": "initialized", "params": {} }))?;
                send(
                    &mut stdin,
                    json!({
                        "method": "thread/start",
                        "id": 1,
                        "params": {
                            "cwd": prepared.worktree,
                            "runtimeWorkspaceRoots": [prepared.worktree],
                            "approvalPolicy": "never",
                            "sandbox": "danger-full-access"
                        }
                    }),
                )?;
            }
            Some(1) => {
                let id = message
                    .pointer("/result/thread/id")
                    .and_then(Value::as_str)
                    .ok_or_else(|| {
                        AgentError::Codex("thread/start response missing thread id".to_owned())
                    })?
                    .to_owned();
                thread_id = Some(id.clone());
                send_turn_start(&mut stdin, config, &id, prepared)?;
            }
            Some(2) => {
                turn_id = message
                    .pointer("/result/turn/id")
                    .and_then(Value::as_str)
                    .map(str::to_owned);
            }
            _ => {}
        }

        if pending_state_query == response_id {
            pending_state_query = None;
            if let Some(turn_id) = &turn_id {
                match turn_status_from_query(&message, turn_id) {
                    Some("completed") => {
                        terminate_child(&mut child);
                        return Ok(());
                    }
                    Some("failed") | Some("interrupted") => {
                        let detail = turn_error_from_query(&message, turn_id)
                            .unwrap_or("turn did not complete successfully");
                        terminate_child(&mut child);
                        return Err(AgentError::Codex(format!(
                            "turn status was {} from state query: {detail}",
                            turn_status_from_query(&message, turn_id).unwrap_or("unknown")
                        )));
                    }
                    Some("inProgress") => {
                        last_observed_method = "turn-state:inProgress".to_owned();
                    }
                    Some(other) => {
                        terminate_child(&mut child);
                        return Err(AgentError::Codex(format!(
                            "turn-state query returned unknown status '{other}'. See {}",
                            event_log_path.display()
                        )));
                    }
                    None => {
                        last_observed_method = format!("turn-state:notFound:{turn_id}");
                    }
                }
            }
        }

        if message.get("method").and_then(Value::as_str) == Some("turn/started") {
            turn_started = true;
        }

        if message.get("method").and_then(Value::as_str) == Some("turn/completed") {
            let status = message
                .pointer("/params/turn/status")
                .and_then(Value::as_str)
                .unwrap_or("unknown");
            terminate_child(&mut child);
            if status == "completed" {
                return Ok(());
            }
            let detail = message
                .pointer("/params/turn/error/message")
                .and_then(Value::as_str)
                .unwrap_or("turn did not complete successfully");
            return Err(AgentError::Codex(format!(
                "turn status was {status}: {detail}"
            )));
        }
    }
}

fn base_command(command: &[String], prepared: &PreparedRun) -> Command {
    let mut process = Command::new(&command[0]);
    process
        .args(&command[1..])
        .current_dir(&prepared.worktree)
        .env("HARNESS_REPO_ROOT", &prepared.worktree)
        .env("HARNESS_DB_PATH", &prepared.harness_db_path)
        .env("HARNESS_RUN_ID", &prepared.run_id)
        .env("HARNESS_RUN_MODE", "execute");
    process
}

fn send(stdin: &mut impl Write, message: Value) -> Result<(), AgentError> {
    writeln!(stdin, "{message}")?;
    stdin.flush()?;
    Ok(())
}

fn append_event_log(path: &Path, line: &str) -> Result<(), AgentError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    writeln!(file, "{line}")?;
    Ok(())
}

fn send_turn_start(
    stdin: &mut impl Write,
    config: &ResolvedConfig,
    thread_id: &str,
    prepared: &PreparedRun,
) -> Result<(), AgentError> {
    send(
        stdin,
        json!({
            "method": "turn/start",
            "id": 2,
            "params": {
                "threadId": thread_id,
                "cwd": prepared.worktree,
                "runtimeWorkspaceRoots": [prepared.worktree],
                "approvalPolicy": "never",
                "sandboxPolicy": { "type": "dangerFullAccess" },
                "input": [
                    {
                        "type": "text",
                        "text": codex_prompt(config, prepared)?,
                        "text_elements": []
                    }
                ]
            }
        }),
    )
}

fn send_turn_state_query(
    stdin: &mut impl Write,
    request_id: i64,
    thread_id: &str,
) -> Result<(), AgentError> {
    send(
        stdin,
        json!({
            "method": "thread/turns/list",
            "id": request_id,
            "params": {
                "threadId": thread_id,
                "limit": 10,
                "sortDirection": "desc",
                "itemsView": "notLoaded"
            }
        }),
    )
}

fn turn_status_from_query<'a>(message: &'a Value, turn_id: &str) -> Option<&'a str> {
    message
        .pointer("/result/data")
        .and_then(Value::as_array)?
        .iter()
        .find(|turn| turn.get("id").and_then(Value::as_str) == Some(turn_id))?
        .get("status")
        .and_then(Value::as_str)
}

fn turn_error_from_query<'a>(message: &'a Value, turn_id: &str) -> Option<&'a str> {
    message
        .pointer("/result/data")
        .and_then(Value::as_array)?
        .iter()
        .find(|turn| turn.get("id").and_then(Value::as_str) == Some(turn_id))?
        .pointer("/error/message")
        .and_then(Value::as_str)
}

fn codex_prompt(config: &ResolvedConfig, prepared: &PreparedRun) -> Result<String, AgentError> {
    let executable = resolve_executable(config.harness_cli.as_deref(), &config.repo_root)?;
    let harness_command = serde_json::to_string(&json!({
        "executable": executable,
        "argv": Vec::<String>::new(),
    }))?;
    Ok(format!(
        "You are running inside a Harness Symphony worktree. Read AGENTS.md and the run contract at {}. Complete only story {} for run {}. Do not change unrelated product code. Write all required artifacts under the current working directory: .harness/runs/{}/SUMMARY.md and .harness/runs/{}/RESULT.json. Use the structured Harness CLI command {} by executing its executable directly and appending each required CLI argument to argv; never shell-split or substitute a fixed repository path. Keep the current worktree as cwd and use HARNESS_DB_PATH, HARNESS_RUN_ID, and HARNESS_RUN_MODE from the environment so .harness/changesets/{}.changeset.jsonl is produced in this worktree. RESULT.json must have version 1, run_id {}, story_id {}, an allowed outcome, summary_path .harness/runs/{}/SUMMARY.md, and a top-level validation object. Do not write validation_evidence. validation must be either {{\"commands\":[{{\"command\":\"exact command\",\"result\":\"pass\"}}]}} with each result set to pass, fail, or unavailable, or {{\"unavailable\":\"non-empty reason\"}}.",
        prepared.contract_path.display(),
        prepared.story_id,
        prepared.run_id,
        prepared.run_id,
        prepared.run_id,
        harness_command,
        prepared.run_id,
        prepared.run_id,
        prepared.story_id,
        prepared.run_id
    ))
}

fn terminate_child(child: &mut std::process::Child) {
    let _ = child.kill();
    let _ = child.wait();
}

fn read_child_stderr(stderr: std::process::ChildStderr) -> Result<String, AgentError> {
    let mut reader = BufReader::new(stderr);
    let mut text = String::new();
    use std::io::Read;
    reader.read_to_string(&mut text)?;
    Ok(text.trim().to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ResolvedConfig;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::path::Path;

    fn install_protocol_cli(temp: &tempfile::TempDir, config: &mut ResolvedConfig) {
        let cli = temp.path().join("fake-harness-cli");
        fs::write(&cli, r#"#!/bin/sh
printf '%s\n' '{"protocol_version":1,"operation":"query.contract","request_id":null,"result":{"protocol_version":1,"cli_version":"0.1.14","schema_minimum":1,"schema_maximum":13,"database_state":"current","database_schema_version":13,"required_environment_variables":["HARNESS_DB_PATH"],"capabilities":["stories.read.v1","stories.write.v1","work-graph.read.v1","story-dependencies.read-write.v1","story-hierarchy.read-write.v1","changesets.apply.v1","changesets.status-sha.v1","isolated-db.v1","isolated-db-snapshot.v1","semantic-operation-log.v1"]},"error":null}'
"#).unwrap();
        let mut permissions = fs::metadata(&cli).unwrap().permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&cli, permissions).unwrap();
        config.harness_cli = Some(cli);
        config.repo_root = temp.path().to_path_buf();
        config.harness_db = temp.path().join("harness.db");
    }

    fn config(adapter: &str, command: Vec<&str>) -> ResolvedConfig {
        ResolvedConfig {
            version: 1,
            repo_root: Path::new("/repo").to_path_buf(),
            harness_db: Path::new("/repo/harness.db").to_path_buf(),
            harness_cli: None,
            state_db: Path::new("/repo/.symphony/state.db").to_path_buf(),
            runs_dir: Path::new("/repo/.harness/runs").to_path_buf(),
            worktrees_dir: Path::new("/repo/.symphony/worktrees").to_path_buf(),
            single_active_run: true,
            agent_adapter: adapter.to_owned(),
            agent_command: command.into_iter().map(str::to_owned).collect(),
            agent_timeout_minutes: 120,
            pull_request_create: "ask".to_owned(),
            pull_request_provider: "github".to_owned(),
            pull_request_draft_for: vec![],
            changeset_directory: Path::new("/repo/.harness/changesets").to_path_buf(),
            changeset_render_in_summary: true,
            allow_here_for_tiny: true,
            compact_keep_last: 50,
            keep_failed_worktrees: true,
            cleanup_after_sync: false,
            auto_source: "harness-db".to_owned(),
            auto_poll_interval_seconds: 30,
            auto_max_attempts: 3,
        }
    }

    fn prepared() -> PreparedRun {
        PreparedRun {
            run_id: "run_1".to_owned(),
            story_id: "US-046".to_owned(),
            branch: Some("symphony/run_1".to_owned()),
            worktree: Path::new("/repo/.symphony/worktrees/run_1").to_path_buf(),
            contract_path: Path::new("/repo/.harness/runs/run_1/RUN_CONTRACT.json").to_path_buf(),
            harness_db_path: Path::new("/repo/.symphony/worktrees/run_1/harness.db").to_path_buf(),
            lightweight: false,
        }
    }

    #[test]
    fn codex_adapter_defaults_to_app_server_command() {
        let config = config("codex", vec![]);

        assert_eq!(
            resolved_agent_command(&config),
            vec!["codex".to_owned(), "app-server".to_owned()]
        );
    }

    #[test]
    fn available_codex_adapter_reports_selected_command() {
        let executable = std::env::current_exe().unwrap().display().to_string();
        let config = config("codex", vec![&executable]);

        assert!(agent_adapter_status(&config)
            .unwrap()
            .contains("codex app-server"));
    }

    #[test]
    fn custom_adapter_requires_command() {
        let config = config("custom", vec![]);

        assert!(matches!(
            agent_adapter_status(&config).unwrap_err(),
            AgentError::MissingCommand
        ));
    }

    #[test]
    fn selected_missing_agent_executable_fails_preflight() {
        let config = config("custom", vec!["symphony-agent-that-does-not-exist"]);

        assert!(matches!(
            preflight_agent(&config).unwrap_err(),
            AgentError::UnavailableCommand(command)
                if command == "symphony-agent-that-does-not-exist"
        ));
    }

    #[cfg(unix)]
    #[test]
    fn selected_non_executable_agent_file_fails_preflight() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("not-executable-agent");
        fs::write(&path, "fixture").unwrap();
        let executable = path.display().to_string();
        let config = config("custom", vec![&executable]);

        assert!(matches!(
            preflight_agent(&config).unwrap_err(),
            AgentError::UnavailableCommand(command) if command == executable
        ));
    }

    #[cfg(unix)]
    #[test]
    fn agent_process_receives_explicit_worktree_repository_environment() {
        let temp = tempfile::tempdir().unwrap();
        let worktree = temp.path().join("worktree with spaces");
        fs::create_dir_all(&worktree).unwrap();
        let prepared = PreparedRun {
            run_id: "run_env".to_owned(),
            story_id: "US-ENV".to_owned(),
            branch: None,
            worktree: worktree.clone(),
            contract_path: worktree.join("RUN_CONTRACT.json"),
            harness_db_path: worktree.join("harness.db"),
            lightweight: false,
        };
        let output = base_command(
            &[
                "sh".to_owned(),
                "-c".to_owned(),
                "printf '%s\\n%s\\n%s\\n' \"$PWD\" \"$HARNESS_REPO_ROOT\" \"$HARNESS_DB_PATH\""
                    .to_owned(),
            ],
            &prepared,
        )
        .output()
        .unwrap();

        assert!(output.status.success());
        let lines = String::from_utf8(output.stdout).unwrap();
        let values = lines.lines().collect::<Vec<_>>();
        assert_eq!(values.len(), 3);
        assert_eq!(
            fs::canonicalize(values[0]).unwrap(),
            fs::canonicalize(&worktree).unwrap()
        );
        assert_eq!(values[1], worktree.to_string_lossy());
        assert_eq!(values[2], prepared.harness_db_path.to_string_lossy());
    }

    #[test]
    fn codex_prompt_points_to_worktree_artifacts_and_run_env() {
        let temp_dir = tempfile::tempdir().unwrap();
        let cli = temp_dir.path().join("Harness CLI with spaces");
        fs::write(&cli, "#!/bin/sh\n").unwrap();
        let mut permissions = fs::metadata(&cli).unwrap().permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&cli, permissions).unwrap();
        let mut config = config("codex", vec![]);
        config.repo_root = temp_dir.path().to_path_buf();
        config.harness_cli = Some(cli.clone());
        let prompt = codex_prompt(&config, &prepared()).unwrap();

        assert!(prompt.contains("US-046"));
        assert!(prompt.contains(".harness/runs/run_1/SUMMARY.md"));
        assert!(prompt.contains(".harness/changesets/run_1.changeset.jsonl"));
        assert!(prompt.contains("\"executable\""));
        assert!(prompt.contains("\"argv\":[]"));
        assert!(prompt.contains("Harness CLI with spaces"));
        assert!(!prompt.contains("scripts/bin/harness-cli"));
        assert!(prompt.contains("HARNESS_DB_PATH"));
        assert!(prompt.contains("top-level validation object"));
        assert!(prompt.contains("Do not write validation_evidence"));
        assert!(prompt.contains("\"result\":\"pass\""));
    }

    #[test]
    fn codex_adapter_completes_json_rpc_handshake() {
        let temp_dir = tempfile::tempdir().unwrap();
        let worktree = temp_dir.path().join("worktree");
        fs::create_dir_all(&worktree).unwrap();
        let fake_server = temp_dir.path().join("fake-codex-app-server");
        fs::write(
            &fake_server,
            r#"#!/usr/bin/env sh
read initialize
printf '%s\n' '{"id":0,"result":{"userAgent":"fake","codexHome":"/tmp","platformFamily":"unix","platformOs":"macos"}}'
read initialized
read thread_start
printf '%s\n' '{"id":1,"result":{"thread":{"id":"thr_1"}}}'
printf '%s\n' '{"method":"thread/started","params":{"thread":{"id":"thr_1"}}}'
read turn_start
printf '%s\n' '{"id":2,"result":{}}'
printf '%s\n' '{"method":"turn/completed","params":{"threadId":"thr_1","turn":{"id":"turn_1","items":[],"itemsView":{"type":"complete"},"status":"completed","error":null,"startedAt":1,"completedAt":2,"durationMs":1000}}}'
"#,
        )
        .unwrap();
        let mut permissions = fs::metadata(&fake_server).unwrap().permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&fake_server, permissions).unwrap();

        let mut config = config("codex", vec![fake_server.to_str().unwrap()]);
        install_protocol_cli(&temp_dir, &mut config);
        config.agent_timeout_minutes = 1;
        let mut prepared = prepared();
        prepared.worktree = worktree.clone();
        prepared.harness_db_path = worktree.join("harness.db");
        prepared.contract_path = worktree.join(".harness/runs/run_1/RUN_CONTRACT.json");

        run_codex_agent(&config, &prepared).unwrap();
    }

    #[test]
    fn codex_adapter_does_not_use_agent_timeout_as_wall_clock_deadline() {
        let temp_dir = tempfile::tempdir().unwrap();
        let worktree = temp_dir.path().join("worktree");
        fs::create_dir_all(&worktree).unwrap();
        let fake_server = temp_dir.path().join("fake-codex-app-server");
        fs::write(
            &fake_server,
            r#"#!/usr/bin/env sh
read initialize
printf '%s\n' '{"id":0,"result":{"userAgent":"fake","codexHome":"/tmp","platformFamily":"unix","platformOs":"macos"}}'
read initialized
read thread_start
printf '%s\n' '{"id":1,"result":{"thread":{"id":"thr_1"}}}'
read turn_start
printf '%s\n' '{"id":2,"result":{"turn":{"id":"turn_1","items":[],"itemsView":"notLoaded","status":"inProgress","error":null,"startedAt":null,"completedAt":null,"durationMs":null}}}'
printf '%s\n' '{"method":"turn/started","params":{"threadId":"thr_1","turn":{"id":"turn_1","items":[],"itemsView":"notLoaded","status":"inProgress","error":null,"startedAt":1,"completedAt":null,"durationMs":null}}}'
read state_query_one
printf '%s\n' '{"id":3,"result":{"data":[{"id":"turn_1","items":[],"itemsView":"notLoaded","status":"inProgress","error":null,"startedAt":1,"completedAt":null,"durationMs":null}],"nextCursor":null,"backwardsCursor":null}}'
read state_query_two
printf '%s\n' '{"id":4,"result":{"data":[{"id":"turn_1","items":[],"itemsView":"notLoaded","status":"completed","error":null,"startedAt":1,"completedAt":2,"durationMs":1000}],"nextCursor":null,"backwardsCursor":null}}'
"#,
        )
        .unwrap();
        let mut permissions = fs::metadata(&fake_server).unwrap().permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&fake_server, permissions).unwrap();

        let mut config = config("codex", vec![fake_server.to_str().unwrap()]);
        install_protocol_cli(&temp_dir, &mut config);
        config.agent_timeout_minutes = 0;
        let mut prepared = prepared();
        prepared.worktree = worktree.clone();
        prepared.harness_db_path = worktree.join("harness.db");
        prepared.contract_path = worktree.join(".harness/runs/run_1/RUN_CONTRACT.json");

        run_codex_agent(&config, &prepared).unwrap();
    }

    #[test]
    fn codex_adapter_reports_failed_terminal_turn() {
        let temp_dir = tempfile::tempdir().unwrap();
        let worktree = temp_dir.path().join("worktree");
        fs::create_dir_all(&worktree).unwrap();
        let fake_server = temp_dir.path().join("fake-codex-app-server");
        fs::write(
            &fake_server,
            r#"#!/usr/bin/env sh
read initialize
printf '%s\n' '{"id":0,"result":{"userAgent":"fake","codexHome":"/tmp","platformFamily":"unix","platformOs":"macos"}}'
read initialized
read thread_start
printf '%s\n' '{"id":1,"result":{"thread":{"id":"thr_1"}}}'
read turn_start
printf '%s\n' '{"id":2,"result":{}}'
printf '%s\n' '{"method":"turn/completed","params":{"threadId":"thr_1","turn":{"id":"turn_1","items":[],"itemsView":{"type":"complete"},"status":"failed","error":{"message":"boom"},"startedAt":1,"completedAt":2,"durationMs":1000}}}'
"#,
        )
        .unwrap();
        let mut permissions = fs::metadata(&fake_server).unwrap().permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&fake_server, permissions).unwrap();

        let mut config = config("codex", vec![fake_server.to_str().unwrap()]);
        install_protocol_cli(&temp_dir, &mut config);
        let mut prepared = prepared();
        prepared.worktree = worktree.clone();
        prepared.harness_db_path = worktree.join("harness.db");
        prepared.contract_path = worktree.join(".harness/runs/run_1/RUN_CONTRACT.json");

        let error = run_codex_agent(&config, &prepared).unwrap_err();
        assert!(error.to_string().contains("turn status was failed: boom"));
    }

    #[test]
    fn codex_adapter_recovers_completed_turn_from_state_query() {
        let temp_dir = tempfile::tempdir().unwrap();
        let worktree = temp_dir.path().join("worktree");
        fs::create_dir_all(&worktree).unwrap();
        let fake_server = temp_dir.path().join("fake-codex-app-server");
        fs::write(
            &fake_server,
            r#"#!/usr/bin/env sh
read initialize
printf '%s\n' '{"id":0,"result":{"userAgent":"fake","codexHome":"/tmp","platformFamily":"unix","platformOs":"macos"}}'
read initialized
read thread_start
printf '%s\n' '{"id":1,"result":{"thread":{"id":"thr_1"}}}'
read turn_start
printf '%s\n' '{"id":2,"result":{"turn":{"id":"turn_1","items":[],"itemsView":"notLoaded","status":"inProgress","error":null,"startedAt":null,"completedAt":null,"durationMs":null}}}'
printf '%s\n' '{"method":"turn/started","params":{"threadId":"thr_1","turn":{"id":"turn_1","items":[],"itemsView":"notLoaded","status":"inProgress","error":null,"startedAt":1,"completedAt":null,"durationMs":null}}}'
read state_query
printf '%s\n' '{"id":3,"result":{"data":[{"id":"turn_1","items":[],"itemsView":"notLoaded","status":"completed","error":null,"startedAt":1,"completedAt":2,"durationMs":1000}],"nextCursor":null,"backwardsCursor":null}}'
"#,
        )
        .unwrap();
        let mut permissions = fs::metadata(&fake_server).unwrap().permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&fake_server, permissions).unwrap();

        let mut config = config("codex", vec![fake_server.to_str().unwrap()]);
        install_protocol_cli(&temp_dir, &mut config);
        config.agent_timeout_minutes = 1;
        let mut prepared = prepared();
        prepared.worktree = worktree.clone();
        prepared.harness_db_path = worktree.join("harness.db");
        prepared.contract_path = worktree.join(".harness/runs/run_1/RUN_CONTRACT.json");

        run_codex_agent(&config, &prepared).unwrap();
    }
}
