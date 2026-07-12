//! Typed process boundary for Harness orchestration protocol v1.
//!
//! This module deliberately knows nothing about Harness' SQLite schema.  A
//! caller selects a repository and database; all Harness state then crosses a
//! bounded JSON subprocess boundary.

use std::collections::BTreeSet;
use std::env;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

use crate::config::ResolvedConfig;

pub const PROTOCOL_VERSION: u32 = 1;
pub const CONTRACT_SCHEMA_MINIMUM: u32 = 1;
pub const CONTRACT_SCHEMA_MAXIMUM: u32 = 13;
pub const SUPPORTED_DATABASE_SCHEMA_MINIMUM: u32 = 12;
pub const OUTPUT_LIMIT_BYTES: usize = 16 * 1024 * 1024;
pub const DEFAULT_READ_TIMEOUT: Duration = Duration::from_secs(30);
pub const DEFAULT_MUTATION_TIMEOUT: Duration = Duration::from_secs(300);
/// Explicitly tested immutable Harness releases. Do not replace this with a
/// semantic-version comparison: protocol capabilities are behavioral promises.
pub const SUPPORTED_CLI_VERSIONS: &[&str] = &["0.1.14"];

pub const REQUIRED_CAPABILITIES: &[&str] = &[
    "stories.read.v1",
    "stories.write.v1",
    "work-graph.read.v1",
    "story-dependencies.read-write.v1",
    "story-hierarchy.read-write.v1",
    "changesets.apply.v1",
    "changesets.status-sha.v1",
    "isolated-db.v1",
    "isolated-db-snapshot.v1",
    "semantic-operation-log.v1",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtocolPolicy {
    pub read_timeout: Duration,
    pub mutation_timeout: Duration,
    pub output_limit: usize,
}

impl Default for ProtocolPolicy {
    fn default() -> Self {
        Self {
            read_timeout: DEFAULT_READ_TIMEOUT,
            mutation_timeout: DEFAULT_MUTATION_TIMEOUT,
            output_limit: OUTPUT_LIMIT_BYTES,
        }
    }
}

#[derive(Debug, Clone)]
pub struct HarnessProtocol {
    executable: PathBuf,
    repo_root: PathBuf,
    harness_db: PathBuf,
    policy: ProtocolPolicy,
    run_context: Option<RunContext>,
}

#[derive(Debug, Clone)]
struct RunContext {
    run_id: String,
    run_mode: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct Contract {
    pub protocol_version: u32,
    pub cli_version: String,
    pub schema_minimum: u32,
    pub schema_maximum: u32,
    pub database_state: DatabaseState,
    pub database_schema_version: Option<u32>,
    pub required_environment_variables: Vec<String>,
    pub capabilities: Vec<String>,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DatabaseState {
    Missing,
    Current,
    NeedsMigration,
    Unsupported,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct Story {
    pub id: String,
    pub title: String,
    pub risk_lane: String,
    pub contract_doc: Option<String>,
    pub status: String,
    pub verify_command: Option<String>,
    pub runnable: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct Dependency {
    pub blocker: String,
    pub blocked: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct Hierarchy {
    pub parent: String,
    pub child: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct WorkGraph {
    pub stories: Vec<Story>,
    pub dependencies: Vec<Dependency>,
    pub hierarchy: Vec<Hierarchy>,
    pub revision: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct SnapshotResult {
    pub output: String,
    pub source_logical_sha256: String,
    pub graph_revision: String,
    pub snapshot_file_sha256: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct StoryCasResult {
    pub id: String,
    pub before_status: String,
    pub after_status: String,
    pub runnable_before: bool,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct ChangesetStatus {
    pub id: String,
    pub content_sha256: String,
    pub applied: bool,
    pub operation_count: u64,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct ChangesetApplyResult {
    pub id: String,
    pub content_sha256: String,
    pub applied: bool,
    pub operations: u64,
}

#[derive(Debug, Clone, Deserialize)]
struct Envelope<T> {
    protocol_version: u32,
    operation: String,
    #[allow(dead_code)]
    request_id: Option<String>,
    result: Option<T>,
    error: Option<ProtocolErrorBody>,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct ProtocolErrorBody {
    pub code: String,
    pub message: String,
    pub retryable: bool,
    #[serde(default)]
    pub details: Value,
}

#[derive(Debug, Error)]
pub enum HarnessProtocolError {
    #[error("Harness CLI not found; configure repo.harness_cli, set HARNESS_CLI_PATH, install the target-local binary, or add harness-cli to PATH")]
    ExecutableNotFound,
    #[error("Harness CLI candidate is not an executable file: {path}; select an installed protocol-v1 CLI")]
    InvalidExecutable { path: PathBuf },
    #[error("failed to start Harness CLI at {path}: {source}; verify the executable and platform")]
    Spawn {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("Harness CLI {operation} timed out after {timeout_seconds}s; inspect logical status before retrying a mutation")]
    Timeout {
        operation: String,
        timeout_seconds: u64,
    },
    #[error("Harness CLI {operation} exceeded the {limit}-byte combined output limit; reduce response size")]
    OutputLimitExceeded { operation: String, limit: usize },
    #[error("Harness CLI {operation} returned non-UTF-8 {stream}; use a protocol-v1 CLI")]
    NonUtf8 {
        operation: String,
        stream: &'static str,
    },
    #[error(
        "Harness CLI {operation} returned malformed JSON: {reason}; upgrade or repair the CLI"
    )]
    MalformedJson { operation: String, reason: String },
    #[error("Harness CLI response for {actual} did not match requested operation {expected}; upgrade or repair the CLI")]
    OperationMismatch { expected: String, actual: String },
    #[error("Harness CLI {operation} returned a result that does not match the request: {reason}")]
    ResultMismatch { operation: String, reason: String },
    #[error("unsupported Harness protocol {actual}, expected {expected}; install the pinned compatible Harness release")]
    ProtocolVersion { expected: u32, actual: u32 },
    #[error("Harness CLI {actual} is not in the tested support set; install harness-cli-v0.1.14 or add an explicitly verified release")]
    UnsupportedCliVersion { actual: String },
    #[error("Harness CLI schema contract {actual_minimum}..={actual_maximum} does not match the tested 1..=13 tuple; install harness-cli-v0.1.14")]
    SchemaContractRange {
        actual_minimum: u32,
        actual_maximum: u32,
    },
    #[error("Harness database at {database} is missing; initialize it explicitly with the compatible Harness CLI")]
    DatabaseMissing { database: PathBuf },
    #[error("Harness database at {database} needs migration; migrate it explicitly, then rerun compatibility discovery")]
    DatabaseNeedsMigration { database: PathBuf },
    #[error("Harness database at {database} is unsupported; select a compatible CLI or restore an approved backup")]
    DatabaseUnsupported { database: PathBuf },
    #[error("Harness CLI is missing capability {capability}; install the pinned compatible Harness release")]
    MissingCapability { capability: String },
    #[error("Harness CLI omitted required environment declaration {name}; install the pinned compatible Harness release")]
    MissingEnvironmentDeclaration { name: String },
    #[error("Harness CLI {operation} failed with exit {exit_code} [{code}]: {message}")]
    Cli {
        operation: String,
        exit_code: i32,
        code: String,
        message: String,
        retryable: bool,
        details: Value,
    },
    #[error("Harness CLI {operation} exited {exit_code} without a valid protocol error envelope; upgrade or repair the CLI")]
    InvalidFailureEnvelope { operation: String, exit_code: i32 },
    #[error("Harness CLI {operation} paired error code {code} with undocumented exit {exit_code}; install a compatible CLI")]
    ExitCodeMismatch {
        operation: String,
        code: String,
        exit_code: i32,
    },
    #[error("failed while collecting Harness CLI {operation} output: {source}")]
    Output {
        operation: String,
        source: std::io::Error,
    },
}

impl HarnessProtocol {
    pub fn from_config(config: &ResolvedConfig) -> Result<Self, HarnessProtocolError> {
        Self::from_configured_path(config, config.harness_cli.as_deref())
    }

    /// Construct from resolved repository settings plus the optional
    /// `repo.harness_cli` value from [`crate::config::SymphonyConfig`]. Keeping
    /// this input explicit avoids hiding executable precedence in consumers.
    pub fn from_configured_path(
        config: &ResolvedConfig,
        configured: Option<&Path>,
    ) -> Result<Self, HarnessProtocolError> {
        let executable = resolve_executable(configured, &config.repo_root)?;
        Ok(Self::new(
            executable,
            config.repo_root.clone(),
            config.harness_db.clone(),
        ))
    }

    pub fn new(executable: PathBuf, repo_root: PathBuf, harness_db: PathBuf) -> Self {
        Self {
            executable,
            repo_root,
            harness_db,
            policy: ProtocolPolicy::default(),
            run_context: None,
        }
    }

    pub fn with_policy(mut self, policy: ProtocolPolicy) -> Self {
        self.policy = policy;
        self
    }

    pub fn with_run_context(
        mut self,
        run_id: impl Into<String>,
        run_mode: impl Into<String>,
    ) -> Self {
        self.run_context = Some(RunContext {
            run_id: run_id.into(),
            run_mode: run_mode.into(),
        });
        self
    }

    pub fn executable(&self) -> &Path {
        &self.executable
    }

    /// Read-only discovery. This method intentionally accepts a missing DB.
    pub fn discover_contract(&self) -> Result<Contract, HarnessProtocolError> {
        self.read("query.contract", ["query", "contract", "--json"])
    }

    /// Full preflight for normal orchestration. No mutation is performed.
    pub fn preflight(&self) -> Result<Contract, HarnessProtocolError> {
        self.preflight_for(REQUIRED_CAPABILITIES)
    }

    pub fn preflight_for(&self, capabilities: &[&str]) -> Result<Contract, HarnessProtocolError> {
        let contract = self.discover_contract()?;
        validate_contract(&contract, &self.harness_db, capabilities)?;
        Ok(contract)
    }

    pub fn work_graph(&self) -> Result<WorkGraph, HarnessProtocolError> {
        self.read("query.work-graph", ["query", "work-graph", "--json"])
    }

    pub fn snapshot(&self, output: &Path) -> Result<SnapshotResult, HarnessProtocolError> {
        let result: SnapshotResult = self.mutate(
            "db.snapshot",
            [
                OsString::from("db"),
                OsString::from("snapshot"),
                OsString::from("--output"),
                output.as_os_str().to_owned(),
                OsString::from("--json"),
            ],
        )?;
        if Path::new(&result.output) != output {
            return Err(HarnessProtocolError::ResultMismatch {
                operation: "db.snapshot".to_owned(),
                reason: format!(
                    "reported output {} instead of {}",
                    result.output,
                    output.display()
                ),
            });
        }
        Ok(result)
    }

    pub fn compare_and_set_status(
        &self,
        id: &str,
        expected_status: &str,
        status: &str,
        require_runnable: bool,
    ) -> Result<StoryCasResult, HarnessProtocolError> {
        let mut args = vec![
            OsString::from("story"),
            OsString::from("update"),
            OsString::from("--id"),
            OsString::from(id),
            OsString::from("--status"),
            OsString::from(status),
            OsString::from("--expected-status"),
            OsString::from(expected_status),
        ];
        if require_runnable {
            args.push(OsString::from("--require-runnable"));
        }
        args.push(OsString::from("--json"));
        let result: StoryCasResult = self.mutate("story.update", args)?;
        if result.id != id
            || result.before_status != expected_status
            || result.after_status != status
            || (require_runnable && !result.runnable_before)
        {
            return Err(HarnessProtocolError::ResultMismatch {
                operation: "story.update".to_owned(),
                reason: "CAS identity/status/runnable fields differ from the request".to_owned(),
            });
        }
        Ok(result)
    }

    pub fn changeset_status(&self, path: &Path) -> Result<ChangesetStatus, HarnessProtocolError> {
        self.read(
            "db.changeset.status",
            [
                OsString::from("db"),
                OsString::from("changeset"),
                OsString::from("status"),
                path.as_os_str().to_owned(),
                OsString::from("--json"),
            ],
        )
    }

    pub fn apply_changeset(
        &self,
        path: &Path,
    ) -> Result<ChangesetApplyResult, HarnessProtocolError> {
        self.mutate(
            "db.changeset.apply",
            [
                OsString::from("db"),
                OsString::from("changeset"),
                OsString::from("apply"),
                path.as_os_str().to_owned(),
                OsString::from("--json"),
            ],
        )
    }

    fn read<T, I, S>(&self, operation: &str, args: I) -> Result<T, HarnessProtocolError>
    where
        T: DeserializeOwned,
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        self.invoke(operation, args, self.policy.read_timeout)
    }

    fn mutate<T, I, S>(&self, operation: &str, args: I) -> Result<T, HarnessProtocolError>
    where
        T: DeserializeOwned,
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        self.invoke(operation, args, self.policy.mutation_timeout)
    }

    fn invoke<T, I, S>(
        &self,
        operation: &str,
        args: I,
        timeout: Duration,
    ) -> Result<T, HarnessProtocolError>
    where
        T: DeserializeOwned,
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let mut command = Command::new(&self.executable);
        command
            .args(args)
            .current_dir(&self.repo_root)
            .env("HARNESS_REPO_ROOT", &self.repo_root)
            .env("HARNESS_DB_PATH", &self.harness_db)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if let Some(context) = &self.run_context {
            command
                .env("HARNESS_RUN_ID", &context.run_id)
                .env("HARNESS_RUN_MODE", &context.run_mode);
        }
        let mut child = command
            .spawn()
            .map_err(|source| HarnessProtocolError::Spawn {
                path: self.executable.clone(),
                source,
            })?;

        let stdout = child.stdout.take().expect("piped stdout");
        let stderr = child.stderr.take().expect("piped stderr");
        let limit = self.policy.output_limit;
        let stdout_thread = thread::spawn(move || read_limited(stdout, limit));
        let stderr_thread = thread::spawn(move || read_limited(stderr, limit));
        let started = Instant::now();
        let status = loop {
            match child.try_wait() {
                Ok(Some(status)) => break status,
                Ok(None) if started.elapsed() < timeout => thread::sleep(Duration::from_millis(10)),
                Ok(None) => {
                    let _ = child.kill();
                    let _ = child.wait();
                    let _ = stdout_thread.join();
                    let _ = stderr_thread.join();
                    return Err(HarnessProtocolError::Timeout {
                        operation: operation.to_owned(),
                        timeout_seconds: timeout.as_secs(),
                    });
                }
                Err(source) => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(HarnessProtocolError::Output {
                        operation: operation.to_owned(),
                        source,
                    });
                }
            }
        };

        let stdout = join_output(stdout_thread, operation)?;
        let stderr = join_output(stderr_thread, operation)?;
        if stdout.len().saturating_add(stderr.len()) > limit {
            return Err(HarnessProtocolError::OutputLimitExceeded {
                operation: operation.to_owned(),
                limit,
            });
        }
        parse_response(operation, status, stdout)
    }
}

pub fn validate_contract(
    contract: &Contract,
    database: &Path,
    required_capabilities: &[&str],
) -> Result<(), HarnessProtocolError> {
    if contract.protocol_version != PROTOCOL_VERSION {
        return Err(HarnessProtocolError::ProtocolVersion {
            expected: PROTOCOL_VERSION,
            actual: contract.protocol_version,
        });
    }
    if !SUPPORTED_CLI_VERSIONS.contains(&contract.cli_version.as_str()) {
        return Err(HarnessProtocolError::UnsupportedCliVersion {
            actual: contract.cli_version.clone(),
        });
    }
    if contract.schema_minimum != CONTRACT_SCHEMA_MINIMUM
        || contract.schema_maximum != CONTRACT_SCHEMA_MAXIMUM
    {
        return Err(HarnessProtocolError::SchemaContractRange {
            actual_minimum: contract.schema_minimum,
            actual_maximum: contract.schema_maximum,
        });
    }
    match contract.database_state {
        DatabaseState::Current
            if contract.database_schema_version.is_some_and(|version| {
                version >= SUPPORTED_DATABASE_SCHEMA_MINIMUM && version <= contract.schema_maximum
            }) => {}
        DatabaseState::Current => {
            return Err(HarnessProtocolError::DatabaseUnsupported {
                database: database.to_owned(),
            });
        }
        DatabaseState::Missing => {
            return Err(HarnessProtocolError::DatabaseMissing {
                database: database.to_owned(),
            });
        }
        DatabaseState::NeedsMigration => {
            return Err(HarnessProtocolError::DatabaseNeedsMigration {
                database: database.to_owned(),
            });
        }
        DatabaseState::Unsupported => {
            return Err(HarnessProtocolError::DatabaseUnsupported {
                database: database.to_owned(),
            });
        }
    }
    if !contract
        .required_environment_variables
        .iter()
        .any(|name| name == "HARNESS_DB_PATH")
    {
        return Err(HarnessProtocolError::MissingEnvironmentDeclaration {
            name: "HARNESS_DB_PATH".to_owned(),
        });
    }
    let present: BTreeSet<&str> = contract.capabilities.iter().map(String::as_str).collect();
    for capability in required_capabilities {
        if !present.contains(capability) {
            return Err(HarnessProtocolError::MissingCapability {
                capability: (*capability).to_owned(),
            });
        }
    }
    Ok(())
}

pub fn resolve_executable(
    configured: Option<&Path>,
    repo_root: &Path,
) -> Result<PathBuf, HarnessProtocolError> {
    if let Some(path) = configured {
        return require_executable(path.to_owned());
    }
    if let Some(path) = env::var_os("HARNESS_CLI_PATH").filter(|value| !value.is_empty()) {
        return require_executable(PathBuf::from(path));
    }
    let local = repo_root
        .join("scripts")
        .join("bin")
        .join(platform_executable_name());
    if is_executable_file(&local) {
        return Ok(local);
    }
    find_on_path(platform_executable_name()).ok_or(HarnessProtocolError::ExecutableNotFound)
}

fn platform_executable_name() -> &'static str {
    if cfg!(windows) {
        "harness-cli.exe"
    } else {
        "harness-cli"
    }
}

fn require_executable(path: PathBuf) -> Result<PathBuf, HarnessProtocolError> {
    if is_executable_file(&path) {
        Ok(path)
    } else {
        Err(HarnessProtocolError::InvalidExecutable { path })
    }
}

fn find_on_path(name: &str) -> Option<PathBuf> {
    env::var_os("PATH").and_then(|paths| {
        env::split_paths(&paths)
            .map(|dir| dir.join(name))
            .find(|candidate| is_executable_file(candidate))
    })
}

fn is_executable_file(path: &Path) -> bool {
    let Ok(metadata) = fs::metadata(path) else {
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
    #[cfg(not(unix))]
    {
        true
    }
}

fn read_limited(mut reader: impl Read, limit: usize) -> std::io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    reader
        .by_ref()
        .take(limit as u64 + 1)
        .read_to_end(&mut bytes)?;
    // Keep draining after overflow so the child cannot deadlock on a full pipe.
    if bytes.len() > limit {
        let mut sink = std::io::sink();
        std::io::copy(&mut reader, &mut sink)?;
    }
    Ok(bytes)
}

fn join_output(
    handle: thread::JoinHandle<std::io::Result<Vec<u8>>>,
    operation: &str,
) -> Result<Vec<u8>, HarnessProtocolError> {
    handle
        .join()
        .map_err(|_| HarnessProtocolError::MalformedJson {
            operation: operation.to_owned(),
            reason: "output reader panicked".to_owned(),
        })?
        .map_err(|source| HarnessProtocolError::Output {
            operation: operation.to_owned(),
            source,
        })
}

fn parse_response<T: DeserializeOwned>(
    operation: &str,
    status: ExitStatus,
    stdout: Vec<u8>,
) -> Result<T, HarnessProtocolError> {
    let text = String::from_utf8(stdout).map_err(|_| HarnessProtocolError::NonUtf8 {
        operation: operation.to_owned(),
        stream: "stdout",
    })?;
    if !text.ends_with('\n') {
        return Err(HarnessProtocolError::MalformedJson {
            operation: operation.to_owned(),
            reason: "machine response is not newline terminated".to_owned(),
        });
    }
    let body = text.strip_suffix('\n').expect("checked trailing newline");
    if body.contains('\n') || body.contains('\r') {
        return Err(HarnessProtocolError::MalformedJson {
            operation: operation.to_owned(),
            reason: "machine response must be exactly one JSON line".to_owned(),
        });
    }
    let envelope: Envelope<T> =
        serde_json::from_str(body).map_err(|error| HarnessProtocolError::MalformedJson {
            operation: operation.to_owned(),
            reason: error.to_string(),
        })?;
    if envelope.protocol_version != PROTOCOL_VERSION {
        return Err(HarnessProtocolError::ProtocolVersion {
            expected: PROTOCOL_VERSION,
            actual: envelope.protocol_version,
        });
    }
    if envelope.operation != operation {
        return Err(HarnessProtocolError::OperationMismatch {
            expected: operation.to_owned(),
            actual: envelope.operation,
        });
    }
    if status.success() {
        match (envelope.result, envelope.error) {
            (Some(result), None) => Ok(result),
            _ => Err(HarnessProtocolError::InvalidFailureEnvelope {
                operation: operation.to_owned(),
                exit_code: 0,
            }),
        }
    } else if let (None, Some(error)) = (envelope.result, envelope.error) {
        let exit_code = status.code().unwrap_or(5);
        let documented_pair = match stable_exit_code(&error.code) {
            Some(expected) => expected == exit_code,
            // Protocol v1 permits additive errors. Their retryability remains
            // authoritative, but they must stay in the documented error range.
            None => (2..=5).contains(&exit_code),
        };
        if !documented_pair {
            return Err(HarnessProtocolError::ExitCodeMismatch {
                operation: operation.to_owned(),
                code: error.code,
                exit_code,
            });
        }
        Err(HarnessProtocolError::Cli {
            operation: operation.to_owned(),
            exit_code,
            code: error.code,
            message: error.message,
            retryable: error.retryable,
            details: error.details,
        })
    } else {
        Err(HarnessProtocolError::InvalidFailureEnvelope {
            operation: operation.to_owned(),
            exit_code: status.code().unwrap_or(5),
        })
    }
}

fn stable_exit_code(code: &str) -> Option<i32> {
    match code {
        "INVALID_ARGUMENT" | "COMPATIBILITY_ERROR" | "PATH_NOT_UTF8" => Some(2),
        "NOT_FOUND" | "CONFLICT" => Some(3),
        "VERIFICATION_FAILED" => Some(4),
        "OUTPUT_LIMIT_EXCEEDED" | "INTERNAL_ERROR" => Some(5),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn contract() -> Contract {
        Contract {
            protocol_version: 1,
            cli_version: "0.1.14".to_owned(),
            schema_minimum: 1,
            schema_maximum: 13,
            database_state: DatabaseState::Current,
            database_schema_version: Some(13),
            required_environment_variables: vec!["HARNESS_DB_PATH".to_owned()],
            capabilities: REQUIRED_CAPABILITIES
                .iter()
                .map(|value| (*value).to_owned())
                .collect(),
        }
    }

    #[test]
    fn validates_full_contract_and_rejects_missing_capability() {
        let mut value = contract();
        validate_contract(&value, Path::new("db"), REQUIRED_CAPABILITIES).unwrap();
        value
            .capabilities
            .retain(|item| item != "isolated-db-snapshot.v1");
        let error = validate_contract(&value, Path::new("db"), REQUIRED_CAPABILITIES).unwrap_err();
        assert!(matches!(
            error,
            HarnessProtocolError::MissingCapability { capability }
                if capability == "isolated-db-snapshot.v1"
        ));
    }

    #[test]
    fn missing_database_is_discoverable_but_not_preflight_compatible() {
        let mut value = contract();
        value.database_state = DatabaseState::Missing;
        assert!(matches!(
            validate_contract(&value, Path::new("harness.db"), &[]),
            Err(HarnessProtocolError::DatabaseMissing { .. })
        ));
    }

    #[test]
    fn legacy_cli_and_inconsistent_current_schema_fail_closed() {
        let mut value = contract();
        value.cli_version = "0.1.11".to_owned();
        assert!(matches!(
            validate_contract(&value, Path::new("harness.db"), &[]),
            Err(HarnessProtocolError::UnsupportedCliVersion { actual }) if actual == "0.1.11"
        ));

        value.cli_version = "0.1.14".to_owned();
        value.database_schema_version = None;
        assert!(matches!(
            validate_contract(&value, Path::new("harness.db"), &[]),
            Err(HarnessProtocolError::DatabaseUnsupported { .. })
        ));
    }

    #[test]
    fn parses_success_and_stable_cli_error_envelopes() {
        #[cfg(unix)]
        use std::os::unix::process::ExitStatusExt;
        #[cfg(windows)]
        use std::os::windows::process::ExitStatusExt;

        let success =
            br#"{"protocol_version":1,"operation":"x","request_id":null,"result":{"value":2}}
"#;
        let value: Value = parse_response("x", ExitStatus::from_raw(0), success.to_vec()).unwrap();
        assert_eq!(value["value"], 2);

        let failure = br#"{"protocol_version":1,"operation":"x","request_id":null,"error":{"code":"CONFLICT","message":"changed","retryable":false,"details":{}}}
"#;
        #[cfg(unix)]
        let failed_status = ExitStatus::from_raw(3 << 8);
        #[cfg(windows)]
        let failed_status = ExitStatus::from_raw(3);
        let error = parse_response::<Value>("x", failed_status, failure.to_vec()).unwrap_err();
        assert!(matches!(
            error,
            HarnessProtocolError::Cli { code, exit_code: 3, .. } if code == "CONFLICT"
        ));
    }

    #[test]
    fn configured_executable_wins_and_invalid_configuration_fails_closed() {
        let temp = tempfile::tempdir().unwrap();
        let executable = temp.path().join("cli with spaces");
        fs::write(&executable, "fixture").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&executable, fs::Permissions::from_mode(0o755)).unwrap();
        }
        assert_eq!(
            resolve_executable(Some(&executable), temp.path()).unwrap(),
            executable
        );
        assert!(matches!(
            resolve_executable(Some(&temp.path().join("missing")), temp.path()),
            Err(HarnessProtocolError::InvalidExecutable { .. })
        ));
    }

    #[test]
    fn output_reader_retains_only_limit_plus_one_bytes() {
        let bytes = vec![b'x'; 128];
        let read = read_limited(bytes.as_slice(), 10).unwrap();
        assert_eq!(read.len(), 11);
    }

    #[cfg(unix)]
    #[test]
    fn process_boundary_passes_argv_cwd_and_explicit_environment() {
        use std::os::unix::fs::PermissionsExt;

        let temp = tempfile::tempdir().unwrap();
        let repo = temp.path().join("repo with spaces");
        fs::create_dir_all(&repo).unwrap();
        let db = repo.join("db with spaces.sqlite");
        let executable = temp.path().join("fake harness cli");
        fs::write(
            &executable,
            r#"#!/bin/sh
set -eu
[ "$(basename "$PWD")" = "repo with spaces" ]
[ -n "$HARNESS_REPO_ROOT" ]
[ "$(basename "$HARNESS_DB_PATH")" = "db with spaces.sqlite" ]
[ "$1 $2 $3" = "query contract --json" ]
echo '{"protocol_version":1,"operation":"query.contract","request_id":"fixture","result":{"protocol_version":1,"cli_version":"0.1.14","schema_minimum":1,"schema_maximum":13,"database_state":"current","database_schema_version":13,"required_environment_variables":["HARNESS_DB_PATH"],"capabilities":["stories.read.v1"]}}'
"#,
        )
        .unwrap();
        fs::set_permissions(&executable, fs::Permissions::from_mode(0o755)).unwrap();

        let protocol = HarnessProtocol::new(executable, repo, db);
        let discovered = protocol.discover_contract().unwrap();
        assert_eq!(discovered.cli_version, "0.1.14");
        assert_eq!(discovered.capabilities, ["stories.read.v1"]);
    }
}
