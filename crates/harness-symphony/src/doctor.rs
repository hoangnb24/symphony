use std::fs;
use std::path::Path;
use std::process::Command;

use thiserror::Error;

use crate::agent::{agent_adapter_status, AgentError};
use crate::config::ResolvedConfig;
use crate::harness_protocol::{Contract, HarnessProtocol, HarnessProtocolError};
use crate::sync::unapplied_changesets;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckStatus {
    Pass,
    Warn,
    Fail,
}

impl CheckStatus {
    fn label(self) -> &'static str {
        match self {
            Self::Pass => "PASS",
            Self::Warn => "WARN",
            Self::Fail => "FAIL",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DoctorCheck {
    pub name: &'static str,
    pub status: CheckStatus,
    pub detail: String,
    pub next: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DoctorReport {
    pub checks: Vec<DoctorCheck>,
}

impl DoctorReport {
    pub fn has_failures(&self) -> bool {
        self.checks
            .iter()
            .any(|check| check.status == CheckStatus::Fail)
    }
}

#[derive(Debug, Error)]
pub enum DoctorError {
    #[error("doctor io error: {0}")]
    Io(#[from] std::io::Error),
}

pub fn run_doctor(config: &ResolvedConfig) -> Result<DoctorReport, DoctorError> {
    let protocol_check = check_harness_protocol(config);
    let protocol_ready = protocol_check.status == CheckStatus::Pass;
    let mut checks = vec![
        check_git_available(),
        check_git_worktree_support(),
        check_repo_root(&config.repo_root),
        check_database_or_changesets(config),
        protocol_check,
        check_gitignore(config),
        check_agent_adapter(config),
        check_pr_adapter(config),
    ];
    if protocol_ready {
        checks.insert(5, check_unapplied_changesets(config));
    }
    Ok(DoctorReport { checks })
}

fn check_harness_protocol(config: &ResolvedConfig) -> DoctorCheck {
    let protocol = match HarnessProtocol::from_config(config) {
        Ok(protocol) => protocol,
        Err(error) => return protocol_failure(error),
    };
    match protocol.preflight() {
        Ok(contract) => protocol_success(protocol.executable(), &contract),
        Err(error) => protocol_failure(error),
    }
}

fn protocol_success(executable: &Path, contract: &Contract) -> DoctorCheck {
    DoctorCheck {
        name: "Harness protocol",
        status: CheckStatus::Pass,
        detail: format!(
            "CLI {} at {}, protocol {}, schema {}, all required capabilities present",
            contract.cli_version,
            executable.display(),
            contract.protocol_version,
            contract
                .database_schema_version
                .map_or_else(|| "unknown".to_owned(), |v| v.to_string())
        ),
        next: None,
    }
}

fn protocol_failure(error: HarnessProtocolError) -> DoctorCheck {
    DoctorCheck {
        name: "Harness protocol",
        status: CheckStatus::Fail,
        detail: error.to_string(),
        next: Some(match error {
            HarnessProtocolError::DatabaseMissing { .. } =>
                "Initialize the database explicitly with harness-cli-v0.1.14, then rerun doctor.".to_owned(),
            HarnessProtocolError::DatabaseNeedsMigration { .. } =>
                "Back up and migrate the database with harness-cli-v0.1.14, then rerun doctor.".to_owned(),
            _ => "Install the checksum-verified harness-cli-v0.1.14 release or configure repo.harness_cli/HARNESS_CLI_PATH, then rerun doctor.".to_owned(),
        }),
    }
}

fn check_unapplied_changesets(config: &ResolvedConfig) -> DoctorCheck {
    match unapplied_changesets(config) {
        Ok(paths) if paths.is_empty() => DoctorCheck {
            name: "changeset sync",
            status: CheckStatus::Pass,
            detail: "all committed changesets are applied locally".to_owned(),
            next: None,
        },
        Ok(paths) => DoctorCheck {
            name: "changeset sync",
            status: CheckStatus::Warn,
            detail: format!("{} committed changeset(s) are unapplied", paths.len()),
            next: Some("Run: harness-symphony sync".to_owned()),
        },
        Err(error) => DoctorCheck {
            name: "changeset sync",
            status: CheckStatus::Warn,
            detail: format!("could not inspect changesets: {error}"),
            next: Some("Run: harness-symphony sync".to_owned()),
        },
    }
}

pub fn print_report(report: &DoctorReport) {
    println!("Harness Symphony Doctor");
    for check in &report.checks {
        println!(
            "[{}] {} - {}",
            check.status.label(),
            check.name,
            check.detail
        );
        if let Some(next) = &check.next {
            println!("  Next: {next}");
        }
    }
}

fn check_git_available() -> DoctorCheck {
    match Command::new("git").arg("--version").output() {
        Ok(output) if output.status.success() => DoctorCheck {
            name: "git",
            status: CheckStatus::Pass,
            detail: String::from_utf8_lossy(&output.stdout).trim().to_owned(),
            next: None,
        },
        _ => DoctorCheck {
            name: "git",
            status: CheckStatus::Fail,
            detail: "git is not available".to_owned(),
            next: Some("Install git and ensure it is on PATH.".to_owned()),
        },
    }
}

fn check_git_worktree_support() -> DoctorCheck {
    match Command::new("git").args(["worktree", "list"]).output() {
        Ok(output) if output.status.success() => DoctorCheck {
            name: "git worktree",
            status: CheckStatus::Pass,
            detail: "git worktree is available".to_owned(),
            next: None,
        },
        _ => DoctorCheck {
            name: "git worktree",
            status: CheckStatus::Fail,
            detail: "git worktree list failed".to_owned(),
            next: Some("Use a Git version that supports worktrees.".to_owned()),
        },
    }
}

fn check_repo_root(repo_root: &Path) -> DoctorCheck {
    match Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .current_dir(repo_root)
        .output()
    {
        Ok(output) if output.status.success() => DoctorCheck {
            name: "repo root",
            status: CheckStatus::Pass,
            detail: String::from_utf8_lossy(&output.stdout).trim().to_owned(),
            next: None,
        },
        _ => DoctorCheck {
            name: "repo root",
            status: CheckStatus::Fail,
            detail: format!("{} is not inside a Git repository", repo_root.display()),
            next: Some(
                "Run harness-symphony from the repository root or pass --repo-root.".to_owned(),
            ),
        },
    }
}

fn check_database_or_changesets(config: &ResolvedConfig) -> DoctorCheck {
    if config.harness_db.exists() {
        return DoctorCheck {
            name: "harness database",
            status: CheckStatus::Pass,
            detail: format!("database exists at {}", config.harness_db.display()),
            next: None,
        };
    }
    if config.changeset_directory.exists() {
        return DoctorCheck {
            name: "harness database",
            status: CheckStatus::Warn,
            detail: "database is absent but changesets are available".to_owned(),
            next: Some(format!(
                "Use the configured Harness CLI executable with argv [\"db\", \"rebuild\", \"--from\", \"{}\"]",
                config.changeset_directory.display()
            )),
        };
    }
    DoctorCheck {
        name: "harness database",
        status: CheckStatus::Fail,
        detail: "harness.db is absent and no changesets directory exists".to_owned(),
        next: Some("Use the configured Harness CLI executable with argv [\"init\"].".to_owned()),
    }
}

fn check_gitignore(config: &ResolvedConfig) -> DoctorCheck {
    let path = config.repo_root.join(".gitignore");
    let Ok(text) = fs::read_to_string(&path) else {
        return DoctorCheck {
            name: ".gitignore",
            status: CheckStatus::Fail,
            detail: ".gitignore is missing".to_owned(),
            next: Some(
                "Add harness.db, harness.db-wal, harness.db-shm, and .symphony/.".to_owned(),
            ),
        };
    };
    let required = [
        "harness.db",
        "harness.db-wal",
        "harness.db-shm",
        ".symphony/",
    ];
    let missing = required
        .iter()
        .filter(|entry| {
            !text
                .lines()
                .map(str::trim)
                .map(|line| line.strip_prefix('/').unwrap_or(line))
                .any(|line| line == **entry)
        })
        .copied()
        .collect::<Vec<_>>();
    if missing.is_empty() {
        DoctorCheck {
            name: ".gitignore",
            status: CheckStatus::Pass,
            detail: "local DB and Symphony runtime files are ignored".to_owned(),
            next: None,
        }
    } else {
        DoctorCheck {
            name: ".gitignore",
            status: CheckStatus::Fail,
            detail: format!("missing ignore entries: {}", missing.join(", ")),
            next: Some(format!("Add to .gitignore: {}", missing.join(", "))),
        }
    }
}

fn check_agent_adapter(config: &ResolvedConfig) -> DoctorCheck {
    match agent_adapter_status(config) {
        Ok(detail) => DoctorCheck {
            name: "agent adapter",
            status: CheckStatus::Pass,
            detail,
            next: None,
        },
        Err(AgentError::MissingCommand) => DoctorCheck {
            name: "agent adapter",
            status: CheckStatus::Warn,
            detail: "custom agent command is not configured".to_owned(),
            next: Some(
                "Set agent.command in .harness/symphony.yml before launching runs.".to_owned(),
            ),
        },
        Err(error) => DoctorCheck {
            name: "agent adapter",
            status: CheckStatus::Fail,
            detail: error.to_string(),
            next: Some("Set agent.adapter to custom or codex in .harness/symphony.yml.".to_owned()),
        },
    }
}

fn check_pr_adapter(config: &ResolvedConfig) -> DoctorCheck {
    if config.pull_request_create == "disabled" || config.pull_request_create == "never" {
        return DoctorCheck {
            name: "PR adapter",
            status: CheckStatus::Warn,
            detail: "PR creation is disabled".to_owned(),
            next: None,
        };
    }
    if config.pull_request_provider == "github" {
        match Command::new("gh").arg("--version").output() {
            Ok(output) if output.status.success() => DoctorCheck {
                name: "PR adapter",
                status: CheckStatus::Pass,
                detail: "GitHub CLI is available".to_owned(),
                next: None,
            },
            _ => DoctorCheck {
                name: "PR adapter",
                status: CheckStatus::Warn,
                detail: "GitHub CLI is not available".to_owned(),
                next: Some("Install gh or set pull_request.create: disabled.".to_owned()),
            },
        }
    } else {
        DoctorCheck {
            name: "PR adapter",
            status: CheckStatus::Warn,
            detail: format!("unsupported PR provider '{}'", config.pull_request_provider),
            next: Some("Set pull_request.provider: github or disable PR creation.".to_owned()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;

    fn base_config() -> ResolvedConfig {
        ResolvedConfig {
            version: 1,
            repo_root: Path::new("/repo").to_path_buf(),
            harness_db: Path::new("/repo/harness.db").to_path_buf(),
            harness_cli: None,
            state_db: Path::new("/repo/.symphony/state.db").to_path_buf(),
            runs_dir: Path::new("/repo/.harness/runs").to_path_buf(),
            worktrees_dir: Path::new("/repo/.symphony/worktrees").to_path_buf(),
            single_active_run: true,
            agent_adapter: "custom".to_owned(),
            agent_command: Vec::new(),
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

    #[test]
    fn report_failure_detection() {
        let report = DoctorReport {
            checks: vec![DoctorCheck {
                name: "x",
                status: CheckStatus::Fail,
                detail: "failed".to_owned(),
                next: Some("fix it".to_owned()),
            }],
        };

        assert!(report.has_failures());
    }

    #[test]
    fn missing_agent_command_is_warning() {
        let config = base_config();
        let check = check_agent_adapter(&config);

        assert_eq!(check.status, CheckStatus::Warn);
        assert!(check.next.unwrap().contains("agent.command"));
    }

    #[test]
    fn unsupported_agent_adapter_fails() {
        let mut config = base_config();
        config.agent_adapter = "unknown".to_owned();
        let check = check_agent_adapter(&config);

        assert_eq!(check.status, CheckStatus::Fail);
        assert!(check.next.unwrap().contains("agent.adapter"));
    }

    #[test]
    fn codex_agent_adapter_passes_without_explicit_command() {
        let mut config = base_config();
        config.agent_adapter = "codex".to_owned();
        let check = check_agent_adapter(&config);

        assert_eq!(check.status, CheckStatus::Pass);
        assert!(check.detail.contains("codex app-server"));
    }

    #[test]
    fn compatible_cli_reports_exact_contract_tuple() {
        let temp = tempfile::tempdir().unwrap();
        let mut config = base_config();
        config.repo_root = temp.path().to_path_buf();
        config.harness_db = temp.path().join("harness.db");
        let cli = temp.path().join("compatible harness cli");
        write_contract_cli(&cli, "0.1.14", true);
        config.harness_cli = Some(cli);

        let check = check_harness_protocol(&config);

        assert_eq!(check.status, CheckStatus::Pass);
        assert!(check.detail.contains("CLI 0.1.14"));
        assert!(check.detail.contains("protocol 1, schema 13"));
    }

    #[test]
    fn incompatible_cli_has_actionable_upgrade_error() {
        let temp = tempfile::tempdir().unwrap();
        let mut config = base_config();
        config.repo_root = temp.path().to_path_buf();
        config.harness_db = temp.path().join("harness.db");
        let cli = temp.path().join("old-cli");
        write_contract_cli(&cli, "0.1.11", true);
        config.harness_cli = Some(cli);

        let check = check_harness_protocol(&config);

        assert_eq!(check.status, CheckStatus::Fail);
        assert!(check.detail.contains("0.1.11"));
        assert!(check.next.unwrap().contains("harness-cli-v0.1.14"));
        assert!(!config.harness_db.exists());
    }

    #[test]
    fn partial_capability_cli_names_missing_capability() {
        let temp = tempfile::tempdir().unwrap();
        let mut config = base_config();
        config.repo_root = temp.path().to_path_buf();
        config.harness_db = temp.path().join("harness.db");
        let cli = temp.path().join("partial-cli");
        write_contract_cli(&cli, "0.1.14", false);
        config.harness_cli = Some(cli);

        let check = check_harness_protocol(&config);

        assert_eq!(check.status, CheckStatus::Fail);
        assert!(check.detail.contains("stories.write.v1"));
        assert!(check.next.unwrap().contains("harness-cli-v0.1.14"));
    }

    fn write_contract_cli(path: &Path, version: &str, complete: bool) {
        let capabilities = if complete {
            crate::harness_protocol::REQUIRED_CAPABILITIES.join("\",\"")
        } else {
            "stories.read.v1".to_owned()
        };
        fs::write(path, format!(r#"#!/bin/sh
printf '%s\n' '{{"protocol_version":1,"operation":"query.contract","request_id":null,"result":{{"protocol_version":1,"cli_version":"{version}","schema_minimum":1,"schema_maximum":13,"database_state":"current","database_schema_version":13,"required_environment_variables":["HARNESS_DB_PATH"],"capabilities":["{capabilities}"]}},"error":null}}'
"#)).unwrap();
        #[cfg(unix)]
        {
            let mut permissions = fs::metadata(path).unwrap().permissions();
            permissions.set_mode(0o755);
            fs::set_permissions(path, permissions).unwrap();
        }
    }
}
