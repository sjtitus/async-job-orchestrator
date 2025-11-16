/*! Jobs module for async orchestrator
 * Defines job structures
 */
use crate::logs;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Immutable metadata (id, type, payload)
/// Mutable runtime fields (state, result, log, created_at, started_at, finished_at)

/**
 * Job state
 */
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "lowercase")]
pub enum State {
    QUEUED,
    RUNNING,
    SUCCEEDED,
    FAILED,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EchoPayload {
    message: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SleepPayload {
    milliseconds: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type", content = "payload")]
#[serde(rename_all = "lowercase")]
pub enum JobSubmission {
    Echo(EchoPayload),
    Sleep(SleepPayload),
}

#[derive(Serialize, Debug, Clone)]
pub struct JobSubmissionResponse {
    id: String,
    job_type: String,
    state: State,
}

impl JobSubmissionResponse {
    pub fn new(req: JobSubmission, init_state: State) -> Self {
        let type_name = match req {
            JobSubmission::Echo(_) => "echo".to_string(),
            JobSubmission::Sleep(_) => "sleep".to_string(),
        };
        Self {
            id: "999".to_string(),
            job_type: type_name,
            state: init_state,
        }
    }
}

// Job: represents a submitted job in-process
pub struct Job {
    id: String,
    job_type: String,
    state: State,
    created_at: DateTime<Utc>,
    started_at: DateTime<Utc>,
    finished_at: DateTime<Utc>,
    result: String,
    log: logs::LogBuffer,
}
