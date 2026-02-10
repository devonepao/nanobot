use crate::cron::schedule::compute_next_run;
use crate::cron::types::{
    CronJob, CronJobState, CronPayload, CronSchedule, CronStore, JobStatus, ScheduleKind,
    ServiceStatus,
};
use anyhow::{Context, Result};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{sleep, Duration};
use tracing::{error, info, warn};

type JobCallback = Arc<dyn Fn(CronJob) -> std::pin::Pin<Box<dyn std::future::Future<Output = Option<String>> + Send>> + Send + Sync>;

pub struct CronService {
    store_path: PathBuf,
    on_job: Option<JobCallback>,
    store: Arc<RwLock<CronStore>>,
    running: Arc<RwLock<bool>>,
}

impl CronService {
    pub fn new(store_path: impl Into<PathBuf>) -> Self {
        Self {
            store_path: store_path.into(),
            on_job: None,
            store: Arc::new(RwLock::new(CronStore::default())),
            running: Arc::new(RwLock::new(false)),
        }
    }

    pub fn with_callback<F, Fut>(mut self, callback: F) -> Self
    where
        F: Fn(CronJob) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Option<String>> + Send + 'static,
    {
        self.on_job = Some(Arc::new(move |job| Box::pin(callback(job))));
        self
    }

    fn now_ms() -> i64 {
        chrono::Utc::now().timestamp_millis()
    }

    async fn load_store(&self) -> Result<()> {
        if !self.store_path.exists() {
            info!("Cron store not found, creating new store");
            return Ok(());
        }

        let content = tokio::fs::read_to_string(&self.store_path)
            .await
            .context("Failed to read cron store")?;

        let store: CronStore =
            serde_json::from_str(&content).context("Failed to parse cron store")?;

        *self.store.write().await = store;
        Ok(())
    }

    async fn save_store(&self) -> Result<()> {
        let store = self.store.read().await;

        if let Some(parent) = self.store_path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .context("Failed to create cron store directory")?;
        }

        let content = serde_json::to_string_pretty(&*store)
            .context("Failed to serialize cron store")?;

        tokio::fs::write(&self.store_path, content)
            .await
            .context("Failed to write cron store")?;

        Ok(())
    }

    async fn recompute_next_runs(&self) {
        let mut store = self.store.write().await;
        let now = Self::now_ms();

        for job in &mut store.jobs {
            if job.enabled {
                job.state.next_run_at_ms = compute_next_run(&job.schedule, now);
            }
        }
    }

    async fn get_next_wake_ms(&self) -> Option<i64> {
        let store = self.store.read().await;
        store
            .jobs
            .iter()
            .filter(|j| j.enabled && j.state.next_run_at_ms.is_some())
            .filter_map(|j| j.state.next_run_at_ms)
            .min()
    }

    async fn execute_job(&self, job_id: &str) {
        let start_ms = Self::now_ms();
        
        let job = {
            let store = self.store.read().await;
            store.jobs.iter().find(|j| j.id == job_id).cloned()
        };

        let Some(mut job) = job else {
            return;
        };

        info!("Cron: executing job '{}' ({})", job.name, job.id);

        match self.run_job_callback(&job).await {
            Ok(_) => {
                job.state.last_status = Some(JobStatus::Ok);
                job.state.last_error = None;
                info!("Cron: job '{}' completed", job.name);
            }
            Err(e) => {
                job.state.last_status = Some(JobStatus::Error);
                job.state.last_error = Some(e.to_string());
                error!("Cron: job '{}' failed: {}", job.name, e);
            }
        }

        job.state.last_run_at_ms = Some(start_ms);
        job.updated_at_ms = Self::now_ms();

        // Handle one-shot jobs
        let should_delete = if job.schedule.kind == ScheduleKind::At {
            if job.delete_after_run {
                true
            } else {
                job.enabled = false;
                job.state.next_run_at_ms = None;
                false
            }
        } else {
            // Compute next run for recurring jobs
            job.state.next_run_at_ms = compute_next_run(&job.schedule, Self::now_ms());
            false
        };

        // Update job in store
        let mut store = self.store.write().await;
        if should_delete {
            store.jobs.retain(|j| j.id != job.id);
        } else {
            if let Some(stored_job) = store.jobs.iter_mut().find(|j| j.id == job.id) {
                *stored_job = job;
            }
        }
    }

    async fn run_job_callback(&self, job: &CronJob) -> Result<()> {
        if let Some(ref callback) = self.on_job {
            let _ = callback(job.clone()).await;
            Ok(())
        } else {
            Ok(())
        }
    }

    async fn on_timer(&self) {
        let now = Self::now_ms();
        
        let due_job_ids: Vec<String> = {
            let store = self.store.read().await;
            store
                .jobs
                .iter()
                .filter(|j| {
                    j.enabled
                        && j.state
                            .next_run_at_ms
                            .map_or(false, |next| now >= next)
                })
                .map(|j| j.id.clone())
                .collect()
        };

        for job_id in due_job_ids {
            self.execute_job(&job_id).await;
        }

        if let Err(e) = self.save_store().await {
            error!("Failed to save cron store: {}", e);
        }
    }

    async fn run_timer_loop(self: Arc<Self>) {
        loop {
            let running = *self.running.read().await;
            if !running {
                break;
            }

            let next_wake = self.get_next_wake_ms().await;
            
            if let Some(next_wake_ms) = next_wake {
                let now = Self::now_ms();
                let delay_ms = (next_wake_ms - now).max(0);
                let delay = Duration::from_millis(delay_ms as u64);

                tokio::select! {
                    _ = sleep(delay) => {
                        let still_running = *self.running.read().await;
                        if still_running {
                            self.on_timer().await;
                        }
                    }
                    _ = tokio::time::sleep(Duration::from_secs(60)) => {
                        // Wake up every minute to check if we're still running
                        continue;
                    }
                }
            } else {
                // No jobs scheduled, sleep for a bit
                sleep(Duration::from_secs(10)).await;
            }
        }
    }

    pub async fn start(&self) -> Result<()> {
        *self.running.write().await = true;

        if let Err(e) = self.load_store().await {
            warn!("Failed to load cron store: {}", e);
        }

        self.recompute_next_runs().await;

        if let Err(e) = self.save_store().await {
            error!("Failed to save cron store: {}", e);
        }

        let job_count = self.store.read().await.jobs.len();
        info!("Cron service started with {} jobs", job_count);

        Ok(())
    }

    pub fn spawn(self: Arc<Self>) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            self.run_timer_loop().await;
        })
    }

    pub async fn stop(&self) {
        *self.running.write().await = false;
        info!("Cron service stopped");
    }

    pub async fn list_jobs(&self, include_disabled: bool) -> Vec<CronJob> {
        let store = self.store.read().await;
        let mut jobs: Vec<CronJob> = if include_disabled {
            store.jobs.clone()
        } else {
            store.jobs.iter().filter(|j| j.enabled).cloned().collect()
        };

        jobs.sort_by_key(|j| j.state.next_run_at_ms.unwrap_or(i64::MAX));
        jobs
    }

    pub async fn add_job(
        &self,
        name: impl Into<String>,
        schedule: CronSchedule,
        message: impl Into<String>,
        deliver: bool,
        channel: Option<String>,
        to: Option<String>,
        delete_after_run: bool,
    ) -> Result<CronJob> {
        let now = Self::now_ms();
        let name = name.into();
        let message = message.into();

        // Generate a short ID (8 chars) for user-friendliness
        // Check for collisions and regenerate if needed
        let job_id = {
            let store = self.store.read().await;
            let mut id = uuid::Uuid::new_v4().to_string()[..8].to_string();
            let mut attempts = 0;
            
            while store.jobs.iter().any(|j| j.id == id) && attempts < 10 {
                id = uuid::Uuid::new_v4().to_string()[..8].to_string();
                attempts += 1;
            }
            
            if attempts == 10 {
                // Fallback to full UUID if we can't find a unique short ID
                uuid::Uuid::new_v4().to_string()
            } else {
                id
            }
        };

        let job = CronJob {
            id: job_id,
            name: name.clone(),
            enabled: true,
            schedule: schedule.clone(),
            payload: CronPayload {
                kind: crate::cron::types::PayloadKind::AgentTurn,
                message,
                deliver,
                channel,
                to,
            },
            state: CronJobState {
                next_run_at_ms: compute_next_run(&schedule, now),
                ..Default::default()
            },
            created_at_ms: now,
            updated_at_ms: now,
            delete_after_run,
        };

        {
            let mut store = self.store.write().await;
            store.jobs.push(job.clone());
        }

        self.save_store().await?;
        info!("Cron: added job '{}' ({})", name, job.id);

        Ok(job)
    }

    pub async fn remove_job(&self, job_id: &str) -> Result<bool> {
        let removed = {
            let mut store = self.store.write().await;
            let before = store.jobs.len();
            store.jobs.retain(|j| j.id != job_id);
            store.jobs.len() < before
        };

        if removed {
            self.save_store().await?;
            info!("Cron: removed job {}", job_id);
        }

        Ok(removed)
    }

    pub async fn enable_job(&self, job_id: &str, enabled: bool) -> Result<Option<CronJob>> {
        let mut store = self.store.write().await;
        
        let job = store.jobs.iter_mut().find(|j| j.id == job_id);
        
        if let Some(job) = job {
            job.enabled = enabled;
            job.updated_at_ms = Self::now_ms();
            
            if enabled {
                job.state.next_run_at_ms = compute_next_run(&job.schedule, Self::now_ms());
            } else {
                job.state.next_run_at_ms = None;
            }
            
            let result = Some(job.clone());
            drop(store);
            self.save_store().await?;
            Ok(result)
        } else {
            Ok(None)
        }
    }

    pub async fn run_job_now(&self, job_id: &str, force: bool) -> Result<bool> {
        let should_run = {
            let store = self.store.read().await;
            store
                .jobs
                .iter()
                .find(|j| j.id == job_id)
                .map(|j| force || j.enabled)
                .unwrap_or(false)
        };

        if should_run {
            self.execute_job(job_id).await;
            self.save_store().await?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub async fn status(&self) -> ServiceStatus {
        let store = self.store.read().await;
        let running = *self.running.read().await;
        
        ServiceStatus {
            enabled: running,
            jobs: store.jobs.len(),
            next_wake_at_ms: store
                .jobs
                .iter()
                .filter(|j| j.enabled && j.state.next_run_at_ms.is_some())
                .filter_map(|j| j.state.next_run_at_ms)
                .min(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_cron_service_lifecycle() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("cron.json");
        
        let service = Arc::new(CronService::new(&store_path));
        service.start().await.unwrap();
        
        let status = service.status().await;
        assert!(status.enabled);
        assert_eq!(status.jobs, 0);
        
        service.stop().await;
    }

    #[tokio::test]
    async fn test_add_and_list_jobs() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("cron.json");
        
        let service = Arc::new(CronService::new(&store_path));
        service.start().await.unwrap();
        
        let schedule = CronSchedule::every(5000);
        service
            .add_job("test-job", schedule, "Hello", false, None, None, false)
            .await
            .unwrap();
        
        let jobs = service.list_jobs(false).await;
        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].name, "test-job");
        
        service.stop().await;
    }

    #[tokio::test]
    async fn test_enable_disable_job() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("cron.json");
        
        let service = Arc::new(CronService::new(&store_path));
        service.start().await.unwrap();
        
        let schedule = CronSchedule::every(5000);
        let job = service
            .add_job("test-job", schedule, "Hello", false, None, None, false)
            .await
            .unwrap();
        
        let updated = service.enable_job(&job.id, false).await.unwrap();
        assert!(updated.is_some());
        assert!(!updated.unwrap().enabled);
        
        let jobs = service.list_jobs(false).await;
        assert_eq!(jobs.len(), 0);
        
        let jobs = service.list_jobs(true).await;
        assert_eq!(jobs.len(), 1);
        
        service.stop().await;
    }
}
