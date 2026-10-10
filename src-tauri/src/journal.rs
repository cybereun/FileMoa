//! Crash-safe run records.
//!
//! Before the first file is moved, the whole plan is written to
//! `<runs>/<run_id>.json` with status `running`. Every settled action is then
//! appended to `<run_id>.journal` (JSON Lines) *before* the next file is
//! touched. When a run finishes, the record is rewritten as `completed` and
//! the journal is removed. If the app dies mid-run, `recover` replays the
//! journal on the next start so the files that did move can still be undone.

use crate::planner::{execute_plan_with, OrganizationPlan, PlannedAction};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};

const SCHEMA_VERSION: u32 = 1;
/// fsync cadence: process crashes lose nothing (writes are unbuffered); this
/// bounds what a power loss can take.
const SYNC_EVERY: usize = 50;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RunRecord {
    schema_version: u32,
    /// `running`, `completed`, or `interrupted`.
    status: String,
    #[serde(default)]
    updated_at: String,
    plan: OrganizationPlan,
}

/// Lightweight row for the history list; the full plan is loaded on demand.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunSummary {
    pub run_id: String,
    pub status: String,
    pub root_path: String,
    pub created_at: String,
    pub updated_at: String,
    pub executed_count: usize,
    pub undone_count: usize,
    pub failed_count: usize,
    pub error_count: usize,
    pub exclusion_count: usize,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct JournalEntry {
    index: usize,
    status: String,
    destination_path: String,
    reason: String,
    collision: bool,
}

pub struct RunJournal {
    record_path: PathBuf,
    journal_path: PathBuf,
    file: File,
    written: usize,
}

fn record_path(dir: &Path, run_id: &str) -> PathBuf {
    dir.join(format!("{run_id}.json"))
}

fn write_record(path: &Path, status: &str, plan: &OrganizationPlan) -> Result<(), String> {
    write_record_at(path, status, plan, &Utc::now().to_rfc3339())
}

fn write_record_at(
    path: &Path,
    status: &str,
    plan: &OrganizationPlan,
    updated_at: &str,
) -> Result<(), String> {
    let record = RunRecord {
        schema_version: SCHEMA_VERSION,
        status: status.into(),
        updated_at: updated_at.into(),
        plan: plan.clone(),
    };
    let encoded = serde_json::to_vec(&record)
        .map_err(|e| format!("실행 기록을 만들 수 없습니다 / cannot encode run record: {e}"))?;
    let temp = path.with_extension("json.tmp");
    let mut file = File::create(&temp)
        .map_err(|e| format!("실행 기록을 저장할 수 없습니다 / cannot write run record: {e}"))?;
    file.write_all(&encoded)
        .and_then(|_| file.sync_all())
        .map_err(|e| format!("실행 기록을 저장할 수 없습니다 / cannot write run record: {e}"))?;
    drop(file);
    // std::fs::rename replaces an existing destination on Windows, which is
    // exactly what an atomic record update needs.
    fs::rename(&temp, path)
        .map_err(|e| format!("실행 기록을 저장할 수 없습니다 / cannot replace run record: {e}"))
}

impl RunJournal {
    /// Persist the plan as `running` and open its journal. Fails before any
    /// file is moved, so an unwritable records folder never leads to an
    /// unrecoverable run.
    pub fn begin(dir: &Path, plan: &mut OrganizationPlan) -> Result<Self, String> {
        fs::create_dir_all(dir).map_err(|e| {
            format!("실행 기록 폴더를 만들 수 없습니다 / cannot create runs folder: {e}")
        })?;
        plan.run_id = uuid::Uuid::new_v4().to_string();
        let record_path = record_path(dir, &plan.run_id);
        write_record(&record_path, "running", plan)?;
        let journal_path = record_path.with_extension("journal");
        let file = OpenOptions::new()
            .create_new(true)
            .append(true)
            .open(&journal_path)
            .map_err(|e| format!("실행 저널을 열 수 없습니다 / cannot open run journal: {e}"))?;
        Ok(Self {
            record_path,
            journal_path,
            file,
            written: 0,
        })
    }

    pub fn record(&mut self, index: usize, action: &PlannedAction) -> Result<(), String> {
        let entry = JournalEntry {
            index,
            status: action.status.clone(),
            destination_path: action.destination_path.clone(),
            reason: action.reason.clone(),
            collision: action.collision,
        };
        let mut line = serde_json::to_vec(&entry).map_err(|e| {
            format!("저널 항목을 만들 수 없습니다 / cannot encode journal entry: {e}")
        })?;
        line.push(b'\n');
        self.file
            .write_all(&line)
            .map_err(|e| format!("저널에 쓸 수 없습니다 / cannot write run journal: {e}"))?;
        self.written += 1;
        if self.written.is_multiple_of(SYNC_EVERY) {
            let _ = self.file.sync_data();
        }
        Ok(())
    }

    pub fn finish(self, plan: &OrganizationPlan) -> Result<(), String> {
        write_record(&self.record_path, "completed", plan)?;
        drop(self.file);
        let _ = fs::remove_file(&self.journal_path);
        Ok(())
    }
}

/// Execute `plan`, journaling each action. If the journal can no longer be
/// written, stop moving files: the remaining actions are skipped rather than
/// executed without a record.
pub fn execute_with_journal(
    dir: &Path,
    mut plan: OrganizationPlan,
) -> Result<OrganizationPlan, String> {
    let mut journal = RunJournal::begin(dir, &mut plan)?;
    let mut journal_error: Option<String> = None;
    let mut plan = execute_plan_with(plan, |index, action| {
        if journal_error.is_some() {
            return Err("journal unavailable".to_string());
        }
        match journal.record(index, action) {
            Ok(()) => Ok(()),
            Err(error) => {
                journal_error = Some(error.clone());
                Err(error)
            }
        }
    })?;
    if let Some(error) = journal_error {
        plan.errors.push(error);
    }
    journal.finish(&plan)?;
    Ok(plan)
}

fn apply_journal(plan: &mut OrganizationPlan, journal_path: &Path) {
    let Ok(contents) = fs::read_to_string(journal_path) else {
        return;
    };
    for line in contents.lines() {
        // A torn last line from a crash is expected; ignore anything unreadable.
        let Ok(entry) = serde_json::from_str::<JournalEntry>(line) else {
            continue;
        };
        let Some(action) = plan.actions.get_mut(entry.index) else {
            continue;
        };
        action.status = entry.status;
        action.destination_path = entry.destination_path;
        action.reason = entry.reason;
        action.collision = entry.collision;
        if action.status == "failed" {
            let message = format!("{}: {}", action.source_path, action.reason);
            plan.errors.push(message);
        }
    }
}

/// Run ids become file names, so only well-formed UUIDs are accepted.
fn checked_record_path(dir: &Path, run_id: &str) -> Result<PathBuf, String> {
    uuid::Uuid::parse_str(run_id)
        .map_err(|_| "잘못된 실행 기록 ID입니다 / invalid run id".to_string())?;
    Ok(record_path(dir, run_id))
}

fn read_record_file(path: &Path) -> Option<RunRecord> {
    let contents = fs::read_to_string(path).ok()?;
    let record = serde_json::from_str::<RunRecord>(&contents).ok()?;
    (record.schema_version == SCHEMA_VERSION).then_some(record)
}

fn parse_time(value: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|time| time.with_timezone(&Utc))
}

fn record_time(record: &RunRecord) -> Option<DateTime<Utc>> {
    parse_time(&record.updated_at).or_else(|| parse_time(&record.plan.created_at))
}

/// Finished runs for the history panel, newest first. Runs older than
/// `retention_days` are deleted here, matching the 30-day Undo window.
pub fn list_runs(dir: &Path, retention_days: i64) -> Result<Vec<RunSummary>, String> {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => {
            return Err(format!(
                "실행 기록 폴더를 읽을 수 없습니다 / cannot read runs folder: {error}"
            ))
        }
    };
    let cutoff = Utc::now() - Duration::days(retention_days);
    let mut rows: Vec<(DateTime<Utc>, RunSummary)> = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|value| value.to_str()) != Some("json") {
            continue;
        }
        let Some(record) = read_record_file(&path) else {
            continue;
        };
        // A `running` record belongs to a live run (or awaits recovery).
        if record.status == "running" {
            continue;
        }
        let time = record_time(&record);
        if time.is_some_and(|time| time < cutoff) {
            let _ = fs::remove_file(&path);
            continue;
        }
        let count = |status: &str| {
            record
                .plan
                .actions
                .iter()
                .filter(|action| action.status == status)
                .count()
        };
        rows.push((
            time.unwrap_or(DateTime::<Utc>::UNIX_EPOCH),
            RunSummary {
                run_id: record.plan.run_id.clone(),
                status: record.status.clone(),
                root_path: record.plan.root_path.clone(),
                created_at: record.plan.created_at.clone(),
                updated_at: record.updated_at.clone(),
                executed_count: count("executed"),
                undone_count: count("undone"),
                failed_count: count("failed"),
                error_count: record.plan.errors.len(),
                exclusion_count: record.plan.exclusions.len(),
            },
        ));
    }
    rows.sort_by(|left, right| right.0.cmp(&left.0));
    Ok(rows.into_iter().map(|(_, row)| row).collect())
}

pub fn load_run(dir: &Path, run_id: &str) -> Result<OrganizationPlan, String> {
    let path = checked_record_path(dir, run_id)?;
    read_record_file(&path)
        .map(|record| record.plan)
        .ok_or_else(|| "실행 기록을 찾을 수 없습니다 / run record not found".to_string())
}

pub fn delete_run(dir: &Path, run_id: &str) -> Result<(), String> {
    let path = checked_record_path(dir, run_id)?;
    if read_record_file(&path).is_some_and(|record| record.status == "running") {
        return Err(
            "진행 중인 실행 기록은 삭제할 수 없습니다 / a run in progress cannot be deleted".into(),
        );
    }
    match fs::remove_file(&path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!(
            "실행 기록을 삭제할 수 없습니다 / cannot delete run record: {error}"
        )),
    }
}

/// Persist a finished run's new state (for example after an undo). Plans
/// that were never recorded are ignored.
pub fn save_finished(dir: &Path, plan: &OrganizationPlan) -> Result<(), String> {
    if plan.run_id.is_empty() {
        return Ok(());
    }
    let path = checked_record_path(dir, &plan.run_id)?;
    let status = read_record_file(&path)
        .map(|record| record.status)
        .unwrap_or_else(|| "completed".into());
    if status == "running" {
        return Err(
            "진행 중인 실행 기록은 갱신할 수 없습니다 / a run in progress cannot be updated".into(),
        );
    }
    fs::create_dir_all(dir).map_err(|e| {
        format!("실행 기록 폴더를 만들 수 없습니다 / cannot create runs folder: {e}")
    })?;
    write_record(&path, &status, plan)
}

/// One-time import of history that older versions kept in the WebView's
/// localStorage. Existing records are never overwritten.
pub fn import_legacy(dir: &Path, plans: Vec<OrganizationPlan>) -> Result<usize, String> {
    fs::create_dir_all(dir).map_err(|e| {
        format!("실행 기록 폴더를 만들 수 없습니다 / cannot create runs folder: {e}")
    })?;
    let mut imported = 0;
    for mut plan in plans {
        if uuid::Uuid::parse_str(&plan.run_id).is_err() {
            plan.run_id = uuid::Uuid::new_v4().to_string();
        }
        let path = record_path(dir, &plan.run_id);
        if path.exists() {
            continue;
        }
        let updated_at = if parse_time(&plan.created_at).is_some() {
            plan.created_at.clone()
        } else {
            Utc::now().to_rfc3339()
        };
        write_record_at(&path, "completed", &plan, &updated_at)?;
        imported += 1;
    }
    Ok(imported)
}

/// Close out runs that were still `running` when the app last stopped.
/// Returns the recovered plans so the UI can offer to review or undo them.
pub fn recover(dir: &Path) -> Result<Vec<OrganizationPlan>, String> {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => {
            return Err(format!(
                "실행 기록 폴더를 읽을 수 없습니다 / cannot read runs folder: {error}"
            ))
        }
    };
    let mut recovered = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|value| value.to_str()) != Some("json") {
            continue;
        }
        let Ok(contents) = fs::read_to_string(&path) else {
            continue;
        };
        let Ok(mut record) = serde_json::from_str::<RunRecord>(&contents) else {
            continue;
        };
        if record.schema_version != SCHEMA_VERSION || record.status != "running" {
            continue;
        }
        let journal_path = path.with_extension("journal");
        apply_journal(&mut record.plan, &journal_path);
        for action in &mut record.plan.actions {
            if action.status == "proposed" {
                action.status = "skipped".into();
                action.reason =
                    "앱이 중단되어 실행되지 않음 / not executed: the app stopped mid-run".into();
            }
        }
        write_record(&path, "interrupted", &record.plan)?;
        let _ = fs::remove_file(&journal_path);
        recovered.push(record.plan);
    }
    Ok(recovered)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::planner::undo_plan;
    use tempfile::tempdir;

    fn action(source: &Path, destination: &Path) -> PlannedAction {
        PlannedAction {
            source_path: source.display().to_string(),
            destination_path: destination.display().to_string(),
            category: "Test".into(),
            reason: "test".into(),
            status: "proposed".into(),
            size_bytes: 1,
            modified_unix_secs: None,
            collision: false,
        }
    }

    fn plan_for(root: &Path, actions: Vec<PlannedAction>) -> OrganizationPlan {
        OrganizationPlan {
            root_path: root.display().to_string(),
            actions,
            exclusions: Vec::new(),
            errors: Vec::new(),
            warnings: Vec::new(),
            created_at: String::new(),
            basis: "type".into(),
            language: "en".into(),
            mode: "safe".into(),
            root_paths: vec![root.display().to_string()],
            run_id: String::new(),
        }
    }

    fn read_record(dir: &Path, run_id: &str) -> RunRecord {
        let text = fs::read_to_string(record_path(dir, run_id)).expect("record");
        serde_json::from_str(&text).expect("valid record")
    }

    #[test]
    fn completed_run_is_recorded_and_journal_removed() {
        let root = tempdir().expect("root");
        let runs = tempdir().expect("runs");
        let source = root.path().join("a.txt");
        fs::write(&source, b"a").expect("source");
        let destination = root.path().join("Text").join("a.txt");
        let plan = plan_for(root.path(), vec![action(&source, &destination)]);
        let done = execute_with_journal(runs.path(), plan).expect("run");
        assert_eq!(done.actions[0].status, "executed");
        assert!(destination.is_file());
        let record = read_record(runs.path(), &done.run_id);
        assert_eq!(record.status, "completed");
        assert!(!record_path(runs.path(), &done.run_id)
            .with_extension("journal")
            .exists());
        assert!(recover(runs.path()).expect("recover").is_empty());
    }

    #[test]
    fn interrupted_run_is_recovered_and_can_be_undone() {
        let root = tempdir().expect("root");
        let runs = tempdir().expect("runs");
        let first = root.path().join("one.txt");
        let second = root.path().join("two.txt");
        fs::write(&first, b"1").expect("first");
        fs::write(&second, b"2").expect("second");
        let first_destination = root.path().join("Text").join("one.txt");
        let second_destination = root.path().join("Text").join("two.txt");
        let mut plan = plan_for(
            root.path(),
            vec![
                action(&first, &first_destination),
                action(&second, &second_destination),
            ],
        );

        // Simulate a crash after the first move: journal it, then never finish.
        let mut journal = RunJournal::begin(runs.path(), &mut plan).expect("begin");
        fs::create_dir_all(first_destination.parent().unwrap()).expect("folder");
        fs::rename(&first, &first_destination).expect("move one");
        plan.actions[0].status = "executed".into();
        journal.record(0, &plan.actions[0]).expect("journal");
        let run_id = plan.run_id.clone();
        drop(journal);
        // A torn trailing line must not break recovery.
        let journal_path = record_path(runs.path(), &run_id).with_extension("journal");
        let mut torn = OpenOptions::new()
            .append(true)
            .open(&journal_path)
            .expect("open journal");
        torn.write_all(b"{\"index\":1,\"stat").expect("torn write");
        drop(torn);

        let recovered = recover(runs.path()).expect("recover");
        assert_eq!(recovered.len(), 1);
        assert_eq!(recovered[0].actions[0].status, "executed");
        assert_eq!(recovered[0].actions[1].status, "skipped");
        assert_eq!(read_record(runs.path(), &run_id).status, "interrupted");
        assert!(recover(runs.path()).expect("second recover").is_empty());

        let undone = undo_plan(recovered.into_iter().next().unwrap()).expect("undo");
        assert_eq!(undone.actions[0].status, "undone");
        assert!(first.is_file());
        assert_eq!(fs::read(&first).unwrap(), b"1");
        assert!(second.is_file());
    }

    #[test]
    fn unwritable_records_folder_blocks_the_run_before_any_move() {
        let root = tempdir().expect("root");
        let blocker = root.path().join("not-a-folder");
        fs::write(&blocker, b"x").expect("blocker");
        let source = root.path().join("a.txt");
        fs::write(&source, b"a").expect("source");
        let destination = root.path().join("Text").join("a.txt");
        let plan = plan_for(root.path(), vec![action(&source, &destination)]);
        assert!(execute_with_journal(&blocker.join("runs"), plan).is_err());
        assert!(source.is_file());
        assert!(!destination.exists());
    }

    fn finished_plan(root: &Path, created_at: String) -> OrganizationPlan {
        let mut plan = plan_for(root, Vec::new());
        plan.created_at = created_at;
        plan
    }

    #[test]
    fn list_load_delete_and_prune_runs() {
        let root = tempdir().expect("root");
        let runs = tempdir().expect("runs");
        let source = root.path().join("a.txt");
        fs::write(&source, b"a").expect("source");
        let destination = root.path().join("Text").join("a.txt");
        let plan = plan_for(root.path(), vec![action(&source, &destination)]);
        let done = execute_with_journal(runs.path(), plan).expect("run");

        // A stale record beyond the retention window is pruned on listing.
        let old = (Utc::now() - Duration::days(40)).to_rfc3339();
        assert_eq!(
            import_legacy(runs.path(), vec![finished_plan(root.path(), old)]).expect("import"),
            1
        );

        let rows = list_runs(runs.path(), 30).expect("list");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].run_id, done.run_id);
        assert_eq!(rows[0].executed_count, 1);
        assert_eq!(
            load_run(runs.path(), &done.run_id)
                .expect("load")
                .actions
                .len(),
            1
        );

        delete_run(runs.path(), &done.run_id).expect("delete");
        assert!(list_runs(runs.path(), 30).expect("list").is_empty());
        assert!(load_run(runs.path(), &done.run_id).is_err());
    }

    #[test]
    fn run_ids_that_are_not_uuids_are_rejected() {
        let runs = tempdir().expect("runs");
        assert!(load_run(runs.path(), "..\\..\\evil").is_err());
        assert!(delete_run(runs.path(), "../x").is_err());
    }

    #[test]
    fn undo_state_is_persisted_and_running_records_are_protected() {
        let root = tempdir().expect("root");
        let runs = tempdir().expect("runs");
        let source = root.path().join("a.txt");
        fs::write(&source, b"a").expect("source");
        let destination = root.path().join("Text").join("a.txt");
        let plan = plan_for(root.path(), vec![action(&source, &destination)]);
        let done = execute_with_journal(runs.path(), plan).expect("run");

        let undone = undo_plan(done).expect("undo");
        save_finished(runs.path(), &undone).expect("save");
        let reloaded = load_run(runs.path(), &undone.run_id).expect("load");
        assert_eq!(reloaded.actions[0].status, "undone");
        assert_eq!(list_runs(runs.path(), 30).unwrap()[0].undone_count, 1);

        let mut live = plan_for(root.path(), Vec::new());
        let _journal = RunJournal::begin(runs.path(), &mut live).expect("begin");
        assert!(delete_run(runs.path(), &live.run_id).is_err());
        assert!(save_finished(runs.path(), &live).is_err());
    }

    #[test]
    fn legacy_import_keeps_existing_records_and_assigns_ids() {
        let root = tempdir().expect("root");
        let runs = tempdir().expect("runs");
        let now = Utc::now().to_rfc3339();
        let plans = vec![
            finished_plan(root.path(), now.clone()),
            finished_plan(root.path(), now),
        ];
        assert_eq!(import_legacy(runs.path(), plans).expect("first"), 2);
        let rows = list_runs(runs.path(), 30).expect("list");
        assert_eq!(rows.len(), 2);
        assert_ne!(rows[0].run_id, rows[1].run_id);
        let again = vec![load_run(runs.path(), &rows[0].run_id).expect("load")];
        assert_eq!(import_legacy(runs.path(), again).expect("second"), 0);
    }
}
