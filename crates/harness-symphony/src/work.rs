use std::collections::{HashMap, HashSet};
use std::path::Path;

use thiserror::Error;

use crate::harness_protocol::{HarnessProtocol, HarnessProtocolError, Story};
use crate::state::{RunRecord, RunStateStore, StateError};

#[derive(Debug, Error)]
pub enum WorkError {
    #[error("story {0} not found")]
    StoryNotFound(String),
    #[error("{0}")]
    Protocol(#[from] HarnessProtocolError),
    #[error("{0}")]
    State(#[from] StateError),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkItem {
    pub id: String,
    pub status: String,
    pub lane: String,
    pub verify: String,
    pub runnable: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkCandidate {
    pub story_id: String,
    pub source: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoardItem {
    pub id: String,
    pub title: String,
    pub story_status: String,
    pub lane: String,
    pub verify: String,
    pub board_state: BoardState,
    pub reason: String,
    pub blockers: Vec<String>,
    pub unblocks: Vec<String>,
    pub parent_id: Option<String>,
    pub children: Vec<String>,
    pub hierarchy_depth: usize,
    pub run_id: Option<String>,
    pub active_run: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BoardState {
    Ready,
    Blocked,
    InProgress,
    Review,
    NeedsAttention,
    Done,
}

impl BoardState {
    pub fn label(&self) -> &'static str {
        match self {
            BoardState::Ready => "Ready",
            BoardState::Blocked => "Blocked",
            BoardState::InProgress => "In Progress",
            BoardState::Review => "Review",
            BoardState::NeedsAttention => "Needs Attention",
            BoardState::Done => "Done",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct StoryRow {
    id: String,
    title: String,
    status: String,
    lane: String,
    verify_command: Option<String>,
}

pub trait WorkSource {
    fn name(&self) -> &'static str;
    fn poll(&self) -> Result<Vec<WorkCandidate>, WorkError>;
}

pub struct HarnessDbWorkSource<'a> {
    protocol: &'a HarnessProtocol,
}

impl<'a> HarnessDbWorkSource<'a> {
    pub fn new(protocol: &'a HarnessProtocol) -> Self {
        Self { protocol }
    }
}

impl WorkSource for HarnessDbWorkSource<'_> {
    fn name(&self) -> &'static str {
        "harness-db"
    }

    fn poll(&self) -> Result<Vec<WorkCandidate>, WorkError> {
        Ok(list_work(self.protocol)?
            .into_iter()
            .filter(is_auto_eligible)
            .map(|item| WorkCandidate {
                story_id: item.id,
                source: self.name().to_owned(),
            })
            .collect())
    }
}

pub const EXTERNAL_WORK_SOURCE_BOUNDARIES: &[&str] =
    &["github-issues", "linear", "jira", "remote-harness"];

pub fn list_work(protocol: &HarnessProtocol) -> Result<Vec<WorkItem>, WorkError> {
    let mut items = protocol
        .work_graph()?
        .stories
        .into_iter()
        .map(|story| {
            classify(
                story.id,
                story.status,
                story.risk_lane,
                story.verify_command,
                story.runnable,
            )
        })
        .collect::<Vec<_>>();
    items.sort_by(|left, right| left.id.cmp(&right.id));
    Ok(items)
}

pub fn list_board(
    protocol: &HarnessProtocol,
    state_db: &Path,
) -> Result<Vec<BoardItem>, WorkError> {
    let graph = protocol.work_graph()?;
    let stories = graph
        .stories
        .into_iter()
        .map(StoryRow::from)
        .collect::<Vec<_>>();
    let stories = stories
        .into_iter()
        .filter(|story| story.status != "retired")
        .collect::<Vec<_>>();
    let story_ids = stories
        .iter()
        .map(|story| story.id.clone())
        .collect::<HashSet<_>>();
    let dependencies = graph
        .dependencies
        .into_iter()
        .map(|edge| (edge.blocker, edge.blocked))
        .filter(|(blocker, blocked)| story_ids.contains(blocker) && story_ids.contains(blocked))
        .collect::<Vec<_>>();
    let blockers_by_story = blockers_by_story(&dependencies);
    let unblocks_by_story = unblocks_by_story(&dependencies);
    let hierarchy = graph
        .hierarchy
        .into_iter()
        .map(|edge| (edge.parent, edge.child))
        .filter(|(parent, child)| story_ids.contains(parent) && story_ids.contains(child))
        .collect::<Vec<_>>();
    let parent_by_child = parent_by_child(&hierarchy);
    let children_by_parent = children_by_parent(&hierarchy);
    let cycle_members = cycle_members(&story_ids, &dependencies);
    let runs = latest_runs_by_story(RunStateStore::new(state_db.to_path_buf()).list_runs()?);

    let done_ids = stories
        .iter()
        .filter(|story| {
            story.status == "implemented"
                || runs
                    .get(&story.id)
                    .is_some_and(|run| run.status == "completed" && is_synced(run))
        })
        .map(|story| story.id.clone())
        .collect::<HashSet<_>>();

    let mut items = stories
        .into_iter()
        .map(|story| {
            let blockers = sorted_vec(
                blockers_by_story
                    .get(&story.id)
                    .cloned()
                    .unwrap_or_default(),
            );
            let unblocks = sorted_vec(
                unblocks_by_story
                    .get(&story.id)
                    .cloned()
                    .unwrap_or_default(),
            );
            let run = runs.get(&story.id);
            let in_cycle = cycle_members.contains(&story.id);
            let parent_id = parent_by_child.get(&story.id).cloned();
            let children = sorted_vec(
                children_by_parent
                    .get(&story.id)
                    .cloned()
                    .unwrap_or_default(),
            );
            let hierarchy_depth = hierarchy_depth(&story.id, &parent_by_child);
            let derivation = BoardDerivation {
                blockers,
                unblocks,
                parent_id,
                children,
                hierarchy_depth,
                in_cycle,
                run,
                done_ids: &done_ids,
            };
            derive_board_item(story, derivation)
        })
        .collect::<Vec<_>>();
    items.sort_by(|left, right| left.id.cmp(&right.id));
    Ok(items)
}

pub fn retire_story(protocol: &HarnessProtocol, story_id: &str) -> Result<(), WorkError> {
    let graph = protocol.work_graph()?;
    let story = graph
        .stories
        .iter()
        .find(|story| story.id == story_id)
        .ok_or_else(|| WorkError::StoryNotFound(story_id.to_owned()))?;
    protocol.compare_and_set_status(story_id, &story.status, "retired", true)?;
    Ok(())
}

fn classify(
    id: String,
    status: String,
    lane: String,
    verify_command: Option<String>,
    protocol_runnable: bool,
) -> WorkItem {
    let has_verify = verify_command
        .as_deref()
        .map(str::trim)
        .is_some_and(|value| !value.is_empty());
    let verify = if has_verify { "configured" } else { "missing" }.to_owned();
    let (runnable, reason) = match status.as_str() {
        "planned" | "in_progress" if protocol_runnable => ("yes", "ready"),
        "planned" | "in_progress" if !has_verify => ("warn", "proof command missing"),
        "planned" | "in_progress" => ("no", "not runnable by Harness protocol"),
        "implemented" => ("no", "already implemented"),
        "retired" => ("no", "retired"),
        "changed" => ("warn", "changed story needs human review"),
        _ => ("no", "unknown story status"),
    };

    WorkItem {
        id,
        status,
        lane,
        verify,
        runnable: runnable.to_owned(),
        reason: reason.to_owned(),
    }
}

impl From<Story> for StoryRow {
    fn from(story: Story) -> Self {
        Self {
            id: story.id,
            title: story.title,
            status: story.status,
            lane: story.risk_lane,
            verify_command: story.verify_command,
        }
    }
}

fn blockers_by_story(edges: &[(String, String)]) -> HashMap<String, HashSet<String>> {
    let mut blockers: HashMap<String, HashSet<String>> = HashMap::new();
    for (blocker, blocked) in edges {
        blockers
            .entry(blocked.clone())
            .or_default()
            .insert(blocker.clone());
    }
    blockers
}

fn unblocks_by_story(edges: &[(String, String)]) -> HashMap<String, HashSet<String>> {
    let mut unblocks: HashMap<String, HashSet<String>> = HashMap::new();
    for (blocker, blocked) in edges {
        unblocks
            .entry(blocker.clone())
            .or_default()
            .insert(blocked.clone());
    }
    unblocks
}

fn parent_by_child(edges: &[(String, String)]) -> HashMap<String, String> {
    let mut parents = HashMap::new();
    for (parent, child) in edges {
        parents
            .entry(child.clone())
            .or_insert_with(|| parent.clone());
    }
    parents
}

fn children_by_parent(edges: &[(String, String)]) -> HashMap<String, HashSet<String>> {
    let mut children: HashMap<String, HashSet<String>> = HashMap::new();
    for (parent, child) in edges {
        children
            .entry(parent.clone())
            .or_default()
            .insert(child.clone());
    }
    children
}

fn hierarchy_depth(story_id: &str, parent_by_child: &HashMap<String, String>) -> usize {
    let mut depth = 0;
    let mut current = story_id;
    let mut seen = HashSet::new();
    while let Some(parent) = parent_by_child.get(current) {
        if !seen.insert(parent.clone()) {
            break;
        }
        depth += 1;
        current = parent;
    }
    depth
}

fn cycle_members(story_ids: &HashSet<String>, edges: &[(String, String)]) -> HashSet<String> {
    let mut graph: HashMap<String, Vec<String>> = HashMap::new();
    for (blocker, blocked) in edges {
        graph
            .entry(blocker.clone())
            .or_default()
            .push(blocked.clone());
    }

    let mut members = HashSet::new();
    for story_id in story_ids {
        let mut stack = Vec::new();
        let mut visited = HashSet::new();
        if reaches(story_id, story_id, &graph, &mut visited, &mut stack) {
            members.extend(stack);
            members.insert(story_id.clone());
        }
    }
    members
}

fn reaches(
    start: &str,
    current: &str,
    graph: &HashMap<String, Vec<String>>,
    visited: &mut HashSet<String>,
    stack: &mut Vec<String>,
) -> bool {
    if !visited.insert(current.to_owned()) {
        return false;
    }
    stack.push(current.to_owned());
    for next in graph.get(current).into_iter().flatten() {
        if next == start || reaches(start, next, graph, visited, stack) {
            return true;
        }
    }
    stack.pop();
    false
}

fn latest_runs_by_story(runs: Vec<RunRecord>) -> HashMap<String, RunRecord> {
    let mut by_story = HashMap::new();
    for run in runs {
        by_story.entry(run.story_id.clone()).or_insert(run);
    }
    by_story
}

struct BoardDerivation<'a> {
    blockers: Vec<String>,
    unblocks: Vec<String>,
    parent_id: Option<String>,
    children: Vec<String>,
    hierarchy_depth: usize,
    in_cycle: bool,
    run: Option<&'a RunRecord>,
    done_ids: &'a HashSet<String>,
}

fn derive_board_item(story: StoryRow, derivation: BoardDerivation<'_>) -> BoardItem {
    let verify = if story
        .verify_command
        .as_deref()
        .map(str::trim)
        .is_some_and(|value| !value.is_empty())
    {
        "configured"
    } else {
        "missing"
    }
    .to_owned();

    let incomplete_blockers = derivation
        .blockers
        .iter()
        .filter(|blocker| !derivation.done_ids.contains(*blocker))
        .cloned()
        .collect::<Vec<_>>();

    let (board_state, reason) = if story.status == "implemented" {
        (BoardState::Done, "story implemented".to_owned())
    } else if story.status == "changed" {
        (
            BoardState::NeedsAttention,
            "changed story needs human review".to_owned(),
        )
    } else if derivation.in_cycle {
        (
            BoardState::Blocked,
            "dependency cycle detected; fix task breakdown".to_owned(),
        )
    } else if let Some(run) = derivation.run {
        match run.status.as_str() {
            "prepared" | "running" => {
                (BoardState::InProgress, format!("active run {}", run.run_id))
            }
            "failed" | "cancelled" | "partial" | "blocked" | "needs_intake" => {
                (BoardState::NeedsAttention, run.next_action.clone())
            }
            "completed" if is_synced(run) => (BoardState::Done, "synced locally".to_owned()),
            "completed" if run.pr_url.is_some() => {
                (BoardState::Review, "review pull request".to_owned())
            }
            "completed" if run.pr_status == "failed" => {
                (BoardState::NeedsAttention, run.next_action.clone())
            }
            "completed" => (
                BoardState::NeedsAttention,
                "completed run is missing required PR review artifact".to_owned(),
            ),
            _ if !incomplete_blockers.is_empty() => (
                BoardState::Blocked,
                format!("waiting for {}", incomplete_blockers.join(", ")),
            ),
            _ => (BoardState::Ready, "ready".to_owned()),
        }
    } else if !incomplete_blockers.is_empty() {
        (
            BoardState::Blocked,
            format!("waiting for {}", incomplete_blockers.join(", ")),
        )
    } else if matches!(story.status.as_str(), "planned" | "in_progress") {
        (BoardState::Ready, "ready".to_owned())
    } else if story.status == "retired" {
        (BoardState::Done, "retired".to_owned())
    } else {
        (
            BoardState::NeedsAttention,
            format!("unknown story status {}", story.status),
        )
    };

    BoardItem {
        id: story.id,
        title: story.title,
        story_status: story.status,
        lane: story.lane,
        verify,
        board_state,
        reason,
        blockers: derivation.blockers,
        unblocks: derivation.unblocks,
        parent_id: derivation.parent_id,
        children: derivation.children,
        hierarchy_depth: derivation.hierarchy_depth,
        run_id: derivation.run.map(|run| run.run_id.clone()),
        active_run: derivation.run.and_then(|run| {
            matches!(run.status.as_str(), "prepared" | "running").then(|| run.run_id.clone())
        }),
    }
}

fn is_synced(run: &RunRecord) -> bool {
    matches!(
        run.sync_status.as_str(),
        "applied" | "synced" | "synced_locally"
    )
}

fn sorted_vec(values: HashSet<String>) -> Vec<String> {
    let mut values = values.into_iter().collect::<Vec<_>>();
    values.sort();
    values
}

fn is_auto_eligible(item: &WorkItem) -> bool {
    item.runnable == "yes" && matches!(item.status.as_str(), "planned" | "in_progress")
}

#[cfg(test)]
mod protocol_tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    #[cfg(unix)]
    fn fake_protocol(temp: &tempfile::TempDir) -> (HarnessProtocol, PathBuf) {
        use std::os::unix::fs::PermissionsExt;

        let repo = temp.path().join("repo");
        fs::create_dir_all(&repo).unwrap();
        let executable = temp.path().join("fake-harness-cli");
        let count = repo.join("invocations");
        fs::write(
            &executable,
            r#"#!/bin/sh
set -eu
printf '1\n' >> "$PWD/invocations"
if [ "$1 $2 $3" = "query work-graph --json" ]; then
  printf '%s\n' '{"protocol_version":1,"operation":"query.work-graph","request_id":"fixture","result":{"stories":[{"id":"US-BLOCKER","title":"blocker","risk_lane":"normal","contract_doc":null,"status":"planned","verify_command":"cargo test","runnable":true},{"id":"US-CHANGED","title":"changed","risk_lane":"normal","contract_doc":null,"status":"changed","verify_command":"cargo test","runnable":false},{"id":"US-DONE","title":"done","risk_lane":"normal","contract_doc":null,"status":"implemented","verify_command":"cargo test","runnable":false}],"dependencies":[{"blocker":"US-BLOCKER","blocked":"US-CHANGED"}],"hierarchy":[],"revision":"fixture-revision"},"error":null}'
elif [ "$1 $2" = "story update" ]; then
  [ "$3 $4 $5 $6 $7 $8 $9 ${10}" = "--id US-BLOCKER --status retired --expected-status planned --require-runnable --json" ]
  printf '%s\n' '{"protocol_version":1,"operation":"story.update","request_id":"fixture","result":{"id":"US-BLOCKER","before_status":"planned","after_status":"retired","runnable_before":true},"error":null}'
else
  exit 64
fi
"#,
        )
        .unwrap();
        fs::set_permissions(&executable, fs::Permissions::from_mode(0o755)).unwrap();
        (
            HarnessProtocol::new(executable, repo.clone(), repo.join("harness.db")),
            count,
        )
    }

    #[test]
    fn changed_classification_requires_human_attention() {
        let item = classify(
            "US-CHANGED".to_owned(),
            "changed".to_owned(),
            "normal".to_owned(),
            Some("cargo test".to_owned()),
            false,
        );
        assert_eq!(item.runnable, "warn");
        assert_eq!(item.reason, "changed story needs human review");
    }

    #[cfg(unix)]
    #[test]
    fn board_uses_one_work_graph_process_and_changed_wins_over_blockers() {
        let temp = tempfile::tempdir().unwrap();
        let (protocol, count) = fake_protocol(&temp);
        let items = list_board(&protocol, &temp.path().join("state.db")).unwrap();
        let changed = items.iter().find(|item| item.id == "US-CHANGED").unwrap();

        assert_eq!(changed.board_state, BoardState::NeedsAttention);
        assert_eq!(changed.reason, "changed story needs human review");
        assert_eq!(fs::read_to_string(count).unwrap().lines().count(), 1);
    }

    #[cfg(unix)]
    #[test]
    fn work_source_fetches_graph_once_not_once_per_story() {
        let temp = tempfile::tempdir().unwrap();
        let (protocol, count) = fake_protocol(&temp);
        let candidates = HarnessDbWorkSource::new(&protocol).poll().unwrap();

        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].story_id, "US-BLOCKER");
        assert_eq!(fs::read_to_string(count).unwrap().lines().count(), 1);
    }

    #[cfg(unix)]
    #[test]
    fn retirement_reads_expected_status_then_uses_cas_mutation() {
        let temp = tempfile::tempdir().unwrap();
        let (protocol, count) = fake_protocol(&temp);
        retire_story(&protocol, "US-BLOCKER").unwrap();

        assert_eq!(fs::read_to_string(count).unwrap().lines().count(), 2);
    }
}
