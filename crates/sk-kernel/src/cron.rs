//! Cron job scheduler engine for the Sovereign Kernel.
//!
//! Ported from OpenFang's openfang-kernel/src/cron.rs — complete with
//! job persistence, CRUD, due-job querying, one-shot/recurring modes,
//! auto-disable on repeated failures, and global/per-agent limits.

use chrono::{Duration, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use sk_types::agent::AgentId;
use sk_types::scheduler::{CronJob, CronJobId, CronSchedule};
use sk_types::{SovereignError, SovereignResult};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use tracing::{debug, info, warn};

/// Maximum consecutive errors before a job is auto-disabled.
const MAX_CONSECUTIVE_ERRORS: u32 = 5;

// ---------------------------------------------------------------------------
// JobMeta — extra runtime state not stored in CronJob itself
// ---------------------------------------------------------------------------

/// Runtime metadata for a cron job that extends the base `CronJob` type.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobMeta {
    /// The underlying job definition.
    pub job: CronJob,
    /// Whether this job should be removed after a single successful execution.
    pub one_shot: bool,
    /// Human-readable status of the last execution.
    pub last_status: Option<String>,
    /// Number of consecutive failed executions.
    pub consecutive_errors: u32,
}

impl JobMeta {
    /// Wrap a `CronJob` with default metadata.
    pub fn new(job: CronJob, one_shot: bool) -> Self {
        Self {
            job,
            one_shot,
            last_status: None,
            consecutive_errors: 0,
        }
    }
}

// ---------------------------------------------------------------------------
// CronScheduler
// ---------------------------------------------------------------------------

/// Cron job scheduler — manages scheduled jobs for all agents.
///
/// Thread-safe via `DashMap`. The kernel should call [`due_jobs`] on a
/// regular interval (e.g. every 10-30 seconds) to discover jobs that need
/// to fire, then call [`record_success`] or [`record_failure`] after
/// execution completes.
pub struct CronScheduler {
    /// All tracked jobs, keyed by their unique ID.
    jobs: DashMap<CronJobId, JobMeta>,
    /// Path to the persistence file.
    persist_path: PathBuf,
    /// Global cap on total jobs across all agents (atomic for hot-reload).
    max_total_jobs: AtomicUsize,
}

impl CronScheduler {
    /// Create a new scheduler.
    ///
    /// `home_dir` is the data directory; jobs are persisted to
    /// `<home_dir>/cron_jobs.json`.
    pub fn new(home_dir: &Path, max_total_jobs: usize) -> Self {
        Self {
            jobs: DashMap::new(),
            persist_path: home_dir.join("cron_jobs.json"),
            max_total_jobs: AtomicUsize::new(max_total_jobs),
        }
    }

    /// Update the max total jobs limit (for hot-reload).
    pub fn set_max_total_jobs(&self, new_max: usize) {
        self.max_total_jobs.store(new_max, Ordering::Relaxed);
    }

    // -- Persistence --------------------------------------------------------

    /// Load persisted jobs from disk.
    pub fn load(&self) -> SovereignResult<usize> {
        if !self.persist_path.exists() {
            return Ok(0);
        }
        let data = std::fs::read_to_string(&self.persist_path)
            .map_err(|e| SovereignError::Internal(format!("Failed to read cron jobs: {e}")))?;
        let metas: Vec<JobMeta> = serde_json::from_str(&data)
            .map_err(|e| SovereignError::Internal(format!("Failed to parse cron jobs: {e}")))?;
        let count = metas.len();
        for meta in metas {
            self.jobs.insert(meta.job.id, meta);
        }
        info!(count, "Loaded cron jobs from disk");
        Ok(count)
    }

    /// Persist all jobs to disk via atomic write.
    pub fn persist(&self) -> SovereignResult<()> {
        let metas: Vec<JobMeta> = self.jobs.iter().map(|r| r.value().clone()).collect();
        let data = serde_json::to_string_pretty(&metas)
            .map_err(|e| SovereignError::Internal(format!("Failed to serialize cron jobs: {e}")))?;
        let tmp_path = self.persist_path.with_extension("json.tmp");
        std::fs::write(&tmp_path, data.as_bytes()).map_err(|e| {
            SovereignError::Internal(format!("Failed to write cron jobs temp file: {e}"))
        })?;
        std::fs::rename(&tmp_path, &self.persist_path).map_err(|e| {
            SovereignError::Internal(format!("Failed to rename cron jobs file: {e}"))
        })?;
        debug!(count = metas.len(), "Persisted cron jobs");
        Ok(())
    }

    // -- CRUD ---------------------------------------------------------------

    /// Add a new job.
    pub fn add_job(&self, mut job: CronJob, one_shot: bool) -> SovereignResult<CronJobId> {
        // Global limit
        let max_jobs = self.max_total_jobs.load(Ordering::Relaxed);
        if self.jobs.len() >= max_jobs {
            return Err(SovereignError::Internal(format!(
                "Global cron job limit reached ({})",
                max_jobs
            )));
        }

        // Per-agent count
        let agent_count = self
            .jobs
            .iter()
            .filter(|r| r.value().job.agent_id == job.agent_id)
            .count();

        job.validate(agent_count)
            .map_err(SovereignError::InvalidInput)?;

        // Compute initial next_run
        job.next_run = Some(compute_next_run(&job.schedule));

        let id = job.id;
        self.jobs.insert(id, JobMeta::new(job, one_shot));
        Ok(id)
    }

    /// Remove a job by ID.
    pub fn remove_job(&self, id: CronJobId) -> SovereignResult<CronJob> {
        self.jobs
            .remove(&id)
            .map(|(_, meta)| meta.job)
            .ok_or_else(|| SovereignError::Internal(format!("Cron job {id} not found")))
    }

    /// Enable or disable a job.
    pub fn set_enabled(&self, id: CronJobId, enabled: bool) -> SovereignResult<()> {
        match self.jobs.get_mut(&id) {
            Some(mut meta) => {
                meta.job.enabled = enabled;
                if enabled {
                    meta.consecutive_errors = 0;
                    meta.job.next_run = Some(compute_next_run(&meta.job.schedule));
                }
                Ok(())
            }
            None => Err(SovereignError::Internal(format!("Cron job {id} not found"))),
        }
    }

    // -- Queries ------------------------------------------------------------

    /// Get a single job by ID.
    pub fn get_job(&self, id: CronJobId) -> Option<CronJob> {
        self.jobs.get(&id).map(|r| r.value().job.clone())
    }

    /// Get the full metadata for a job.
    pub fn get_meta(&self, id: CronJobId) -> Option<JobMeta> {
        self.jobs.get(&id).map(|r| r.value().clone())
    }

    /// List all jobs for a specific agent.
    pub fn list_jobs(&self, agent_id: AgentId) -> Vec<CronJob> {
        self.jobs
            .iter()
            .filter(|r| r.value().job.agent_id == agent_id)
            .map(|r| r.value().job.clone())
            .collect()
    }

    /// List all jobs across all agents.
    pub fn list_all_jobs(&self) -> Vec<CronJob> {
        self.jobs.iter().map(|r| r.value().job.clone()).collect()
    }

    /// Total number of tracked jobs.
    pub fn total_jobs(&self) -> usize {
        self.jobs.len()
    }

    /// Return jobs whose `next_run` is at or before `now` and are enabled.
    pub fn due_jobs(&self) -> Vec<CronJob> {
        let now = Utc::now();
        self.jobs
            .iter()
            .filter(|r| {
                let meta = r.value();
                meta.job.enabled && meta.job.next_run.map(|t| t <= now).unwrap_or(false)
            })
            .map(|r| r.value().job.clone())
            .collect()
    }

    // -- Outcome recording --------------------------------------------------

    /// Record a successful execution for a job.
    pub fn record_success(&self, id: CronJobId) {
        let should_remove = {
            if let Some(mut meta) = self.jobs.get_mut(&id) {
                meta.job.last_run = Some(Utc::now());
                meta.last_status = Some("ok".to_string());
                meta.consecutive_errors = 0;
                if meta.one_shot {
                    true
                } else {
                    meta.job.next_run = Some(compute_next_run(&meta.job.schedule));
                    false
                }
            } else {
                return;
            }
        };
        if should_remove {
            self.jobs.remove(&id);
        }
    }

    /// Record a failed execution for a job.
    pub fn record_failure(&self, id: CronJobId, error_msg: &str) {
        if let Some(mut meta) = self.jobs.get_mut(&id) {
            meta.job.last_run = Some(Utc::now());
            meta.last_status = Some(format!("error: {}", &error_msg[..error_msg.len().min(256)]));
            meta.consecutive_errors += 1;
            if meta.consecutive_errors >= MAX_CONSECUTIVE_ERRORS {
                warn!(
                    job_id = %id,
                    errors = meta.consecutive_errors,
                    "Auto-disabling cron job after repeated failures"
                );
                meta.job.enabled = false;
            } else {
                meta.job.next_run = Some(compute_next_run(&meta.job.schedule));
            }
        }
    }
}

// ---------------------------------------------------------------------------
// compute_next_run
// ---------------------------------------------------------------------------

/// Compute the next fire time for a schedule.
pub fn compute_next_run(schedule: &CronSchedule) -> chrono::DateTime<Utc> {
    match schedule {
        CronSchedule::At { at } => *at,
        CronSchedule::Every { every_secs } => Utc::now() + Duration::seconds(*every_secs as i64),
        CronSchedule::Cron { .. } => {
            // Placeholder: real cron parsing will be added when the `cron`
            // crate is brought in. For now, fire 60 seconds from now.
            Utc::now() + Duration::seconds(60)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;
    use sk_types::scheduler::{CronAction, CronDelivery};

    fn make_job(agent_id: AgentId) -> CronJob {
        CronJob {
            id: CronJobId::new(),
            agent_id,
            name: "test-job".into(),
            enabled: true,
            schedule: CronSchedule::Every { every_secs: 3600 },
            action: CronAction::SystemEvent {
                text: "ping".into(),
            },
            delivery: CronDelivery::None,
            created_at: Utc::now(),
            last_run: None,
            next_run: None,
        }
    }

    fn make_scheduler(max_total: usize) -> (CronScheduler, tempfile::TempDir) {
        let tmp = tempfile::tempdir().unwrap();
        let sched = CronScheduler::new(tmp.path(), max_total);
        (sched, tmp)
    }

    #[test]
    fn test_add_job_and_list() {
        let (sched, _tmp) = make_scheduler(100);
        let agent = AgentId::new();
        let job = make_job(agent);
        let id = sched.add_job(job, false).unwrap();

        let jobs = sched.list_jobs(agent);
        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].id, id);
        assert_eq!(jobs[0].name, "test-job");

        let all = sched.list_all_jobs();
        assert_eq!(all.len(), 1);

        let fetched = sched.get_job(id).unwrap();
        assert_eq!(fetched.agent_id, agent);
        assert!(fetched.next_run.is_some());
        assert_eq!(sched.total_jobs(), 1);
    }

    #[test]
    fn test_remove_job() {
        let (sched, _tmp) = make_scheduler(100);
        let agent = AgentId::new();
        let job = make_job(agent);
        let id = sched.add_job(job, false).unwrap();

        let removed = sched.remove_job(id).unwrap();
        assert_eq!(removed.name, "test-job");
        assert_eq!(sched.total_jobs(), 0);
        assert!(sched.remove_job(id).is_err());
    }

    #[test]
    fn test_add_job_global_limit() {
        let (sched, _tmp) = make_scheduler(2);
        let agent = AgentId::new();

        sched.add_job(make_job(agent), false).unwrap();
        sched.add_job(make_job(agent), false).unwrap();

        let err = sched.add_job(make_job(agent), false).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("limit"), "Expected limit error, got: {msg}");
    }

    #[test]
    fn test_record_success_removes_one_shot() {
        let (sched, _tmp) = make_scheduler(100);
        let agent = AgentId::new();
        let id = sched.add_job(make_job(agent), true).unwrap();

        sched.record_success(id);
        assert_eq!(sched.total_jobs(), 0);
    }

    #[test]
    fn test_record_success_keeps_recurring() {
        let (sched, _tmp) = make_scheduler(100);
        let agent = AgentId::new();
        let id = sched.add_job(make_job(agent), false).unwrap();

        sched.record_success(id);
        assert_eq!(sched.total_jobs(), 1);
        let meta = sched.get_meta(id).unwrap();
        assert_eq!(meta.last_status.as_deref(), Some("ok"));
        assert_eq!(meta.consecutive_errors, 0);
    }

    #[test]
    fn test_record_failure_auto_disable() {
        let (sched, _tmp) = make_scheduler(100);
        let agent = AgentId::new();
        let id = sched.add_job(make_job(agent), false).unwrap();

        for i in 0..(MAX_CONSECUTIVE_ERRORS - 1) {
            sched.record_failure(id, &format!("error {i}"));
            let meta = sched.get_meta(id).unwrap();
            assert!(meta.job.enabled);
        }

        sched.record_failure(id, "final error");
        let meta = sched.get_meta(id).unwrap();
        assert!(!meta.job.enabled);
        assert_eq!(meta.consecutive_errors, MAX_CONSECUTIVE_ERRORS);
    }

    #[test]
    fn test_due_jobs_only_enabled() {
        let (sched, _tmp) = make_scheduler(100);
        let agent = AgentId::new();

        let mut j1 = make_job(agent);
        j1.name = "enabled-due".into();
        let id1 = sched.add_job(j1, false).unwrap();

        let mut j2 = make_job(agent);
        j2.name = "disabled-job".into();
        let id2 = sched.add_job(j2, false).unwrap();
        sched.set_enabled(id2, false).unwrap();

        // Force job 1's next_run to the past
        if let Some(mut meta) = sched.jobs.get_mut(&id1) {
            meta.job.next_run = Some(Utc::now() - Duration::seconds(10));
        }
        // Force job 2's next_run to the past too (but disabled)
        if let Some(mut meta) = sched.jobs.get_mut(&id2) {
            meta.job.next_run = Some(Utc::now() - Duration::seconds(10));
        }

        let due = sched.due_jobs();
        assert_eq!(due.len(), 1);
        assert_eq!(due[0].name, "enabled-due");
    }

    #[test]
    fn test_due_jobs_future_not_included() {
        let (sched, _tmp) = make_scheduler(100);
        let agent = AgentId::new();
        sched.add_job(make_job(agent), false).unwrap();

        let due = sched.due_jobs();
        assert!(due.is_empty());
    }

    #[test]
    fn test_set_enabled() {
        let (sched, _tmp) = make_scheduler(100);
        let agent = AgentId::new();
        let id = sched.add_job(make_job(agent), false).unwrap();

        sched.set_enabled(id, false).unwrap();
        let meta = sched.get_meta(id).unwrap();
        assert!(!meta.job.enabled);

        sched.set_enabled(id, true).unwrap();
        let meta = sched.get_meta(id).unwrap();
        assert!(meta.job.enabled);
        assert_eq!(meta.consecutive_errors, 0);

        let fake_id = CronJobId::new();
        assert!(sched.set_enabled(fake_id, true).is_err());
    }

    #[test]
    fn test_persist_and_load() {
        let tmp = tempfile::tempdir().unwrap();
        let agent = AgentId::new();

        {
            let sched = CronScheduler::new(tmp.path(), 100);
            let mut j1 = make_job(agent);
            j1.name = "persist-a".into();
            let mut j2 = make_job(agent);
            j2.name = "persist-b".into();
            sched.add_job(j1, false).unwrap();
            sched.add_job(j2, true).unwrap();
            sched.persist().unwrap();
        }

        {
            let sched = CronScheduler::new(tmp.path(), 100);
            let count = sched.load().unwrap();
            assert_eq!(count, 2);
            assert_eq!(sched.total_jobs(), 2);

            let jobs = sched.list_jobs(agent);
            let names: Vec<&str> = jobs.iter().map(|j| j.name.as_str()).collect();
            assert!(names.contains(&"persist-a"));
            assert!(names.contains(&"persist-b"));
        }
    }

    #[test]
    fn test_load_no_file_returns_zero() {
        let tmp = tempfile::tempdir().unwrap();
        let sched = CronScheduler::new(tmp.path(), 100);
        assert_eq!(sched.load().unwrap(), 0);
    }

    #[test]
    fn test_compute_next_run_at() {
        let target = Utc::now() + Duration::hours(2);
        let schedule = CronSchedule::At { at: target };
        let next = compute_next_run(&schedule);
        assert_eq!(next, target);
    }

    #[test]
    fn test_compute_next_run_every() {
        let before = Utc::now();
        let schedule = CronSchedule::Every { every_secs: 300 };
        let next = compute_next_run(&schedule);
        let after = Utc::now();
        assert!(next >= before + Duration::seconds(300));
        assert!(next <= after + Duration::seconds(300));
    }

    #[test]
    fn test_record_failure_truncates_long_error() {
        let (sched, _tmp) = make_scheduler(100);
        let agent = AgentId::new();
        let id = sched.add_job(make_job(agent), false).unwrap();

        let long_error = "x".repeat(1000);
        sched.record_failure(id, &long_error);

        let meta = sched.get_meta(id).unwrap();
        let status = meta.last_status.unwrap();
        assert!(status.len() <= 263);
    }
}
