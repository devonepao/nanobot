use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ScheduleKind {
    At,
    Every,
    Cron,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CronSchedule {
    pub kind: ScheduleKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub at_ms: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub every_ms: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expr: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tz: Option<String>,
}

impl CronSchedule {
    pub fn at(timestamp_ms: i64) -> Self {
        Self {
            kind: ScheduleKind::At,
            at_ms: Some(timestamp_ms),
            every_ms: None,
            expr: None,
            tz: None,
        }
    }

    pub fn every(interval_ms: i64) -> Self {
        Self {
            kind: ScheduleKind::Every,
            at_ms: None,
            every_ms: Some(interval_ms),
            expr: None,
            tz: None,
        }
    }

    pub fn cron(expression: impl Into<String>) -> Self {
        Self {
            kind: ScheduleKind::Cron,
            at_ms: None,
            every_ms: None,
            expr: Some(expression.into()),
            tz: None,
        }
    }

    pub fn with_timezone(mut self, tz: impl Into<String>) -> Self {
        self.tz = Some(tz.into());
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PayloadKind {
    SystemEvent,
    AgentTurn,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CronPayload {
    #[serde(default = "default_payload_kind")]
    pub kind: PayloadKind,
    #[serde(default)]
    pub message: String,
    #[serde(default)]
    pub deliver: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channel: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to: Option<String>,
}

fn default_payload_kind() -> PayloadKind {
    PayloadKind::AgentTurn
}

impl Default for CronPayload {
    fn default() -> Self {
        Self {
            kind: PayloadKind::AgentTurn,
            message: String::new(),
            deliver: false,
            channel: None,
            to: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum JobStatus {
    Ok,
    Error,
    Skipped,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CronJobState {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_run_at_ms: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_run_at_ms: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_status: Option<JobStatus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CronJob {
    pub id: String,
    pub name: String,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    pub schedule: CronSchedule,
    #[serde(default)]
    pub payload: CronPayload,
    #[serde(default)]
    pub state: CronJobState,
    #[serde(default)]
    pub created_at_ms: i64,
    #[serde(default)]
    pub updated_at_ms: i64,
    #[serde(default)]
    pub delete_after_run: bool,
}

fn default_enabled() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CronStore {
    #[serde(default = "default_version")]
    pub version: i32,
    #[serde(default)]
    pub jobs: Vec<CronJob>,
}

fn default_version() -> i32 {
    1
}

impl Default for CronStore {
    fn default() -> Self {
        Self {
            version: 1,
            jobs: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceStatus {
    pub enabled: bool,
    pub jobs: usize,
    pub next_wake_at_ms: Option<i64>,
}
