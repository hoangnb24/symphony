use std::path::{Path, PathBuf};
use std::process::Command;

use thiserror::Error;

use crate::changeset::{changeset_files, changeset_id, ChangesetError};
use crate::config::ResolvedConfig;
use crate::harness_protocol::{HarnessProtocol, HarnessProtocolError};
use crate::state::{RunStateStore, StateError};

#[derive(Debug, Error)]
pub enum SyncError {
    #[error("{0}")]
    Changeset(#[from] ChangesetError),
    #[error("{0}")]
    State(#[from] StateError),
    #[error("{0}")]
    Protocol(#[from] HarnessProtocolError),
    #[error("Harness changeset response mismatch for {path}: {detail}")]
    ResponseMismatch { path: String, detail: String },
    #[error("sync io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("git command failed: {0}")]
    GitFailed(String),
    #[error("checkout has local changes; commit, stash, or reset before syncing:\n{0}")]
    DirtyCheckout(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncChange {
    pub id: String,
    pub path: PathBuf,
    pub applied: bool,
    pub operations: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncResult {
    pub changes: Vec<SyncChange>,
}

pub fn sync_changesets(config: &ResolvedConfig) -> Result<SyncResult, SyncError> {
    let protocol = HarnessProtocol::from_config(config)?;
    protocol.preflight()?;
    let store = RunStateStore::new(config.state_db.clone());
    store.ensure_migration_fence_released()?;
    refresh_checkout_from_upstream(config)?;
    let protocol = HarnessProtocol::from_config(config)?;
    protocol.preflight()?;
    store.ensure_migration_fence_released()?;
    store.init()?;
    let paths = changeset_files(&config.changeset_directory)?;
    let mut changes = Vec::new();
    for path in paths {
        changes.push(apply_changeset_path(&protocol, &store, path)?);
    }
    Ok(SyncResult { changes })
}

pub fn sync_changeset(config: &ResolvedConfig, run_id: &str) -> Result<SyncResult, SyncError> {
    let protocol = HarnessProtocol::from_config(config)?;
    protocol.preflight()?;
    let store = RunStateStore::new(config.state_db.clone());
    store.ensure_migration_fence_released()?;
    refresh_checkout_from_upstream(config)?;
    let protocol = HarnessProtocol::from_config(config)?;
    protocol.preflight()?;
    store.ensure_migration_fence_released()?;
    store.init()?;
    let path = config
        .changeset_directory
        .join(format!("{run_id}.changeset.jsonl"));
    let change = apply_changeset_path(&protocol, &store, path)?;
    Ok(SyncResult {
        changes: vec![change],
    })
}

pub fn refresh_checkout_from_upstream(config: &ResolvedConfig) -> Result<bool, SyncError> {
    if upstream_branch(&config.repo_root)?.is_none() {
        return Ok(false);
    }
    ensure_clean_checkout(&config.repo_root)?;
    git_command(&config.repo_root, &["pull", "--ff-only"])?;
    Ok(true)
}

pub fn unapplied_changesets(config: &ResolvedConfig) -> Result<Vec<PathBuf>, SyncError> {
    let protocol = HarnessProtocol::from_config(config)?;
    protocol.preflight()?;
    let store = RunStateStore::new(config.state_db.clone());
    store.init()?;
    let mut unapplied = Vec::new();
    for path in changeset_files(&config.changeset_directory)? {
        let status = protocol.changeset_status(&path)?;
        if !status.applied || !store.changeset_synced(&status.id, &status.content_sha256)? {
            unapplied.push(path);
        }
    }
    Ok(unapplied)
}

fn apply_changeset_path(
    protocol: &HarnessProtocol,
    store: &RunStateStore,
    path: PathBuf,
) -> Result<SyncChange, SyncError> {
    let guard = store.acquire_migration_fence_guard()?;
    let id = changeset_id(&path)?;
    let status = protocol.changeset_status(&path)?;
    if status.id != id {
        return Err(SyncError::ResponseMismatch {
            path: path.display().to_string(),
            detail: format!("status returned id {} instead of {id}", status.id),
        });
    }
    if status.applied {
        guard.record_changeset_synced(&id, &path, &status.content_sha256, true)?;
        guard.update_sync_status_if_present(&id, "synced", "done")?;
        guard.commit()?;
        return Ok(SyncChange {
            id,
            path,
            applied: true,
            operations: 0,
        });
    }
    let result = protocol.apply_changeset(&path)?;
    if result.id != id {
        return Err(SyncError::ResponseMismatch {
            path: path.display().to_string(),
            detail: format!("apply returned id {} instead of {id}", result.id),
        });
    }
    let durable = protocol.changeset_status(&path)?;
    if durable.id != id || !durable.applied || durable.content_sha256 != result.content_sha256 {
        return Err(SyncError::ResponseMismatch {
            path: path.display().to_string(),
            detail: "post-apply status did not confirm the same applied content SHA".to_owned(),
        });
    }
    let durable_applied = true;
    let operations = usize::try_from(result.operations).unwrap_or(usize::MAX);
    guard.record_changeset_synced(&id, &path, &durable.content_sha256, durable_applied)?;
    guard.update_sync_status_if_present(&id, "synced", "done")?;
    guard.commit()?;
    Ok(SyncChange {
        id,
        path,
        applied: durable_applied,
        operations,
    })
}

fn upstream_branch(repo_root: &Path) -> Result<Option<String>, SyncError> {
    let output = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "--symbolic-full-name", "@{u}"])
        .current_dir(repo_root)
        .output()?;
    if output.status.success() {
        let upstream = String::from_utf8_lossy(&output.stdout).trim().to_owned();
        return Ok((!upstream.is_empty()).then_some(upstream));
    }
    Ok(None)
}

fn ensure_clean_checkout(repo_root: &Path) -> Result<(), SyncError> {
    let output = git_output(
        repo_root,
        &["status", "--porcelain", "--untracked-files=all"],
    )?;
    let status = String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .filter(|line| !is_ignorable_checkout_status(line))
        .collect::<Vec<_>>()
        .join("\n");
    if status.is_empty() {
        Ok(())
    } else {
        Err(SyncError::DirtyCheckout(status))
    }
}

fn is_ignorable_checkout_status(line: &str) -> bool {
    let path = porcelain_path(line);
    path == ".harness/symphony.yml"
        || path.starts_with(".harness/runs/")
        || path.ends_with(".tsbuildinfo")
}

fn porcelain_path(line: &str) -> &str {
    line.get(3..).unwrap_or(line).trim()
}

fn git_command(repo_root: &Path, args: &[&str]) -> Result<(), SyncError> {
    let output = git_output(repo_root, args)?;
    if output.status.success() {
        Ok(())
    } else {
        Err(SyncError::GitFailed(
            String::from_utf8_lossy(&output.stderr).trim().to_owned(),
        ))
    }
}

fn git_output(repo_root: &Path, args: &[&str]) -> Result<std::process::Output, SyncError> {
    Command::new("git")
        .args(args)
        .current_dir(repo_root)
        .output()
        .map_err(SyncError::from)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ResolvedConfig;
    use std::fs;
    use std::process::Command;

    #[test]
    fn compatible_json_cli_applies_only_requested_run() {
        let temp_dir = tempfile::tempdir().unwrap();
        let config = config_for_root(temp_dir.path());
        fs::create_dir_all(&config.changeset_directory).unwrap();
        fs::create_dir_all(temp_dir.path().join("scripts/bin")).unwrap();
        fs::write(
            config.changeset_directory.join("run_one.changeset.jsonl"),
            r#"{"op":"changeset.header","version":1,"run_id":"run_one"}
{"op":"story.update","version":1,"id":"US-ONE","payload":{"status":"implemented"}}
"#,
        )
        .unwrap();
        fs::write(
            config.changeset_directory.join("run_two.changeset.jsonl"),
            r#"{"op":"changeset.header","version":1,"run_id":"run_two"}
{"op":"story.update","version":1,"id":"US-TWO","payload":{"status":"implemented"}}
"#,
        )
        .unwrap();
        let cli_path = temp_dir.path().join("scripts/bin/harness-cli");
        write_fake_cli(&cli_path, true);
        make_executable(&cli_path);

        let result = sync_changeset(&config, "run_one").unwrap();

        assert_eq!(result.changes.len(), 1);
        assert_eq!(result.changes[0].id, "run_one");
        assert!(result.changes[0].applied);
        let args = fs::read_to_string(temp_dir.path().join("sync-args.log")).unwrap();
        assert!(args.contains(".harness/changesets/run_one.changeset.jsonl"));
        assert!(!args.contains("run_two.changeset.jsonl"));
    }

    #[test]
    fn incompatible_cli_fails_before_state_store_write() {
        let temp_dir = tempfile::tempdir().unwrap();
        let config = config_for_root(temp_dir.path());
        fs::create_dir_all(&config.changeset_directory).unwrap();
        fs::create_dir_all(temp_dir.path().join("scripts/bin")).unwrap();
        fs::write(
            config.changeset_directory.join("run_old.changeset.jsonl"),
            "{\"op\":\"changeset.header\",\"version\":1,\"run_id\":\"run_old\"}\n",
        )
        .unwrap();
        write_fake_cli(&temp_dir.path().join("scripts/bin/harness-cli"), false);

        let error = sync_changeset(&config, "run_old").unwrap_err();

        assert!(
            matches!(error, SyncError::Protocol(HarnessProtocolError::UnsupportedCliVersion { actual }) if actual == "0.1.11")
        );
        assert!(!config.state_db.exists());
    }

    #[test]
    fn migration_fence_rejects_sync_before_changeset_status_or_apply() {
        let temp_dir = tempfile::tempdir().unwrap();
        let config = config_for_root(temp_dir.path());
        fs::create_dir_all(&config.changeset_directory).unwrap();
        fs::create_dir_all(temp_dir.path().join("scripts/bin")).unwrap();
        fs::write(
            config
                .changeset_directory
                .join("run_fenced.changeset.jsonl"),
            "{\"op\":\"changeset.header\",\"version\":1,\"run_id\":\"run_fenced\"}\n",
        )
        .unwrap();
        write_fake_cli(&temp_dir.path().join("scripts/bin/harness-cli"), true);
        RunStateStore::new(config.state_db.clone())
            .hold_migration_fence("ownership handoff")
            .unwrap();

        let error = sync_changeset(&config, "run_fenced").unwrap_err();

        assert!(matches!(
            error,
            SyncError::State(StateError::MigrationFenceHeld(reason))
                if reason == "ownership handoff"
        ));
        let args = fs::read_to_string(temp_dir.path().join("sync-args.log")).unwrap();
        assert!(args.contains("query\ncontract\n--json"));
        assert!(!args.contains("status"));
        assert!(!args.contains("apply"));
    }

    #[test]
    fn applied_same_id_with_different_content_sha_fails_closed() {
        let temp_dir = tempfile::tempdir().unwrap();
        let config = config_for_root(temp_dir.path());
        fs::create_dir_all(&config.changeset_directory).unwrap();
        fs::create_dir_all(temp_dir.path().join("scripts/bin")).unwrap();
        let path = config.changeset_directory.join("run_same.changeset.jsonl");
        fs::write(
            &path,
            "{\"op\":\"changeset.header\",\"version\":1,\"run_id\":\"run_same\"}\n",
        )
        .unwrap();
        write_fake_cli(&temp_dir.path().join("scripts/bin/harness-cli"), true);
        fs::write(temp_dir.path().join(".applied-run_same"), "").unwrap();
        RunStateStore::new(config.state_db.clone())
            .record_changeset_synced(
                "run_same",
                &path,
                "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                true,
            )
            .unwrap();

        let error = sync_changeset(&config, "run_same").unwrap_err();

        assert!(matches!(
            error,
            SyncError::State(StateError::ChangesetContentConflict { id, .. })
                if id == "run_same"
        ));
    }

    #[test]
    fn refresh_checkout_fast_forwards_from_upstream() {
        let temp_dir = tempfile::tempdir().unwrap();
        let remote = temp_dir.path().join("remote.git");
        run_git(
            temp_dir.path(),
            &["init", "--bare", &remote.display().to_string()],
        );
        let local = temp_dir.path().join("local");
        let other = temp_dir.path().join("other");
        run_git(
            temp_dir.path(),
            &[
                "clone",
                &remote.display().to_string(),
                &local.display().to_string(),
            ],
        );
        configure_git(&local);
        fs::write(local.join("README.md"), "one\n").unwrap();
        run_git(&local, &["add", "README.md"]);
        run_git(&local, &["commit", "-m", "one"]);
        run_git(&local, &["push", "-u", "origin", "HEAD"]);
        run_git(
            temp_dir.path(),
            &[
                "clone",
                &remote.display().to_string(),
                &other.display().to_string(),
            ],
        );
        configure_git(&other);
        fs::write(other.join("README.md"), "two\n").unwrap();
        run_git(&other, &["commit", "-am", "two"]);
        run_git(&other, &["push"]);

        let refreshed = refresh_checkout_from_upstream(&config_for_root(&local)).unwrap();

        assert!(refreshed);
        assert_eq!(
            fs::read_to_string(local.join("README.md")).unwrap(),
            "two\n"
        );
    }

    #[test]
    fn refresh_checkout_refuses_dirty_checkout() {
        let temp_dir = tempfile::tempdir().unwrap();
        let remote = temp_dir.path().join("remote.git");
        run_git(
            temp_dir.path(),
            &["init", "--bare", &remote.display().to_string()],
        );
        let local = temp_dir.path().join("local");
        run_git(
            temp_dir.path(),
            &[
                "clone",
                &remote.display().to_string(),
                &local.display().to_string(),
            ],
        );
        configure_git(&local);
        fs::write(local.join("README.md"), "one\n").unwrap();
        run_git(&local, &["add", "README.md"]);
        run_git(&local, &["commit", "-m", "one"]);
        run_git(&local, &["push", "-u", "origin", "HEAD"]);
        fs::write(local.join("local.txt"), "dirty\n").unwrap();

        let error = refresh_checkout_from_upstream(&config_for_root(&local)).unwrap_err();

        assert!(matches!(error, SyncError::DirtyCheckout(status) if status.contains("local.txt")));
    }

    #[test]
    fn refresh_checkout_allows_only_local_symphony_artifacts() {
        let temp_dir = tempfile::tempdir().unwrap();
        let remote = temp_dir.path().join("remote.git");
        run_git(
            temp_dir.path(),
            &["init", "--bare", &remote.display().to_string()],
        );
        let local = temp_dir.path().join("local");
        run_git(
            temp_dir.path(),
            &[
                "clone",
                &remote.display().to_string(),
                &local.display().to_string(),
            ],
        );
        configure_git(&local);
        fs::write(local.join("README.md"), "one\n").unwrap();
        run_git(&local, &["add", "README.md"]);
        run_git(&local, &["commit", "-m", "one"]);
        run_git(&local, &["push", "-u", "origin", "HEAD"]);
        fs::create_dir_all(local.join(".harness/runs/run_1")).unwrap();
        fs::write(local.join(".harness/runs/run_1/RESULT.json"), "{}\n").unwrap();
        fs::write(local.join(".harness/symphony.yml"), "version: 1\n").unwrap();

        let refreshed = refresh_checkout_from_upstream(&config_for_root(&local)).unwrap();

        assert!(refreshed);
    }

    #[test]
    fn refresh_checkout_allows_generated_typescript_build_info() {
        let temp_dir = tempfile::tempdir().unwrap();
        let remote = temp_dir.path().join("remote.git");
        run_git(
            temp_dir.path(),
            &["init", "--bare", &remote.display().to_string()],
        );
        let local = temp_dir.path().join("local");
        run_git(
            temp_dir.path(),
            &[
                "clone",
                &remote.display().to_string(),
                &local.display().to_string(),
            ],
        );
        configure_git(&local);
        fs::write(local.join("README.md"), "one\n").unwrap();
        run_git(&local, &["add", "README.md"]);
        run_git(&local, &["commit", "-m", "one"]);
        run_git(&local, &["push", "-u", "origin", "HEAD"]);
        fs::create_dir_all(local.join("crates/harness-symphony/web-ui")).unwrap();
        fs::write(
            local.join("crates/harness-symphony/web-ui/tsconfig.tsbuildinfo"),
            "{}\n",
        )
        .unwrap();

        let refreshed = refresh_checkout_from_upstream(&config_for_root(&local)).unwrap();

        assert!(refreshed);
    }

    #[test]
    fn refresh_checkout_still_refuses_code_changes_with_local_symphony_artifacts() {
        let temp_dir = tempfile::tempdir().unwrap();
        let remote = temp_dir.path().join("remote.git");
        run_git(
            temp_dir.path(),
            &["init", "--bare", &remote.display().to_string()],
        );
        let local = temp_dir.path().join("local");
        run_git(
            temp_dir.path(),
            &[
                "clone",
                &remote.display().to_string(),
                &local.display().to_string(),
            ],
        );
        configure_git(&local);
        fs::write(local.join("README.md"), "one\n").unwrap();
        run_git(&local, &["add", "README.md"]);
        run_git(&local, &["commit", "-m", "one"]);
        run_git(&local, &["push", "-u", "origin", "HEAD"]);
        fs::create_dir_all(local.join(".harness/runs/run_1")).unwrap();
        fs::write(local.join(".harness/runs/run_1/RESULT.json"), "{}\n").unwrap();
        fs::write(local.join(".harness/symphony.yml"), "version: 1\n").unwrap();
        fs::write(local.join("local.txt"), "dirty\n").unwrap();

        let error = refresh_checkout_from_upstream(&config_for_root(&local)).unwrap_err();

        assert!(
            matches!(error, SyncError::DirtyCheckout(status) if status.contains("local.txt") && !status.contains(".harness/runs") && !status.contains(".harness/symphony.yml"))
        );
    }

    #[test]
    fn refresh_checkout_refuses_unapplied_harness_changesets() {
        let temp_dir = tempfile::tempdir().unwrap();
        let remote = temp_dir.path().join("remote.git");
        run_git(
            temp_dir.path(),
            &["init", "--bare", &remote.display().to_string()],
        );
        let local = temp_dir.path().join("local");
        run_git(
            temp_dir.path(),
            &[
                "clone",
                &remote.display().to_string(),
                &local.display().to_string(),
            ],
        );
        configure_git(&local);
        fs::write(local.join("README.md"), "one\n").unwrap();
        run_git(&local, &["add", "README.md"]);
        run_git(&local, &["commit", "-m", "one"]);
        run_git(&local, &["push", "-u", "origin", "HEAD"]);
        fs::create_dir_all(local.join(".harness/changesets")).unwrap();
        fs::write(
            local.join(".harness/changesets/run_1.changeset.jsonl"),
            "{}\n",
        )
        .unwrap();

        let error = refresh_checkout_from_upstream(&config_for_root(&local)).unwrap_err();

        assert!(
            matches!(error, SyncError::DirtyCheckout(status) if status.contains(".harness/changesets"))
        );
    }

    fn configure_git(repo: &Path) {
        run_git(repo, &["config", "user.email", "test@example.invalid"]);
        run_git(repo, &["config", "user.name", "Test User"]);
    }

    fn run_git(repo: &Path, args: &[&str]) {
        let output = Command::new("git")
            .args(args)
            .current_dir(repo)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "git {:?} failed: {}",
            args,
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fn write_fake_cli(path: &Path, compatible: bool) {
        let version = if compatible { "0.1.14" } else { "0.1.11" };
        let script = format!(
            r#"#!/bin/sh
printf '%s\n' "$@" >> sync-args.log
if [ "$1 $2 $3" = "query contract --json" ]; then
  printf '%s\n' '{{"protocol_version":1,"operation":"query.contract","request_id":null,"result":{{"protocol_version":1,"cli_version":"{version}","schema_minimum":1,"schema_maximum":13,"database_state":"current","database_schema_version":13,"required_environment_variables":["HARNESS_DB_PATH"],"capabilities":["stories.read.v1","stories.write.v1","work-graph.read.v1","story-dependencies.read-write.v1","story-hierarchy.read-write.v1","changesets.apply.v1","changesets.status-sha.v1","isolated-db.v1","isolated-db-snapshot.v1","semantic-operation-log.v1"]}},"error":null}}'
elif [ "$3" = "status" ]; then
  id=$(basename "$4" .changeset.jsonl)
  applied=false
  [ -f ".applied-$id" ] && applied=true
  printf '%s\n' "{{\"protocol_version\":1,\"operation\":\"db.changeset.status\",\"request_id\":null,\"result\":{{\"id\":\"$id\",\"content_sha256\":\"0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\",\"applied\":$applied,\"operation_count\":2}},\"error\":null}}"
elif [ "$3" = "apply" ]; then
  id=$(basename "$4" .changeset.jsonl)
  : > ".applied-$id"
  printf '%s\n' "{{\"protocol_version\":1,\"operation\":\"db.changeset.apply\",\"request_id\":null,\"result\":{{\"id\":\"$id\",\"content_sha256\":\"0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\",\"applied\":true,\"operations\":2}},\"error\":null}}"
fi
"#
        );
        fs::write(path, script).unwrap();
        make_executable(path);
    }

    #[cfg(unix)]
    fn make_executable(path: &Path) {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = fs::metadata(path).unwrap().permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(path, permissions).unwrap();
    }

    #[cfg(not(unix))]
    fn make_executable(_path: &Path) {}

    fn config_for_root(root: &Path) -> ResolvedConfig {
        ResolvedConfig {
            version: 1,
            repo_root: root.to_path_buf(),
            harness_db: root.join("harness.db"),
            harness_cli: None,
            state_db: root.join(".symphony/state.db"),
            runs_dir: root.join(".harness/runs"),
            worktrees_dir: root.join(".symphony/worktrees"),
            single_active_run: true,
            agent_adapter: "custom".to_owned(),
            agent_command: vec![],
            agent_timeout_minutes: 120,
            pull_request_create: "ask".to_owned(),
            pull_request_provider: "github".to_owned(),
            pull_request_draft_for: vec![],
            changeset_directory: root.join(".harness/changesets"),
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
}
