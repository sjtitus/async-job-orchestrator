/*! Jobs module for async orchestrator
 * Defines job structures
 */
use std::{collections::VecDeque, sync::Arc};

use crate::logs::{LogBuffer, LogLevel};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::{
    Mutex,
    mpsc::{self, Sender},
};
use ulid::Ulid;

/**
 * Job state
 */
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "lowercase")]
pub enum State {
    INIT,
    QUEUED,
    RUNNING,
    SUCCEEDED,
    FAILED,
}

/**
 * Job payloads for various job types
 */
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EchoPayload {
    message: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SleepPayload {
    milliseconds: u32,
}

/**
 * Job Submission
 * Serializiable structure
 * JSON rep received as payload by API
 */
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type", content = "payload")]
#[serde(rename_all = "lowercase")]
pub enum JobSubmission {
    Echo(EchoPayload),
    Sleep(SleepPayload),
}

/**
 * Job
 * Fundamental internal data structure for a job
 * Includes integrating log
 */
#[derive(Clone)]
pub struct Job {
    id: Ulid,
    submission: JobSubmission,
    state: State,
    created_at: DateTime<Utc>,
    started_at: Option<DateTime<Utc>>,
    finished_at: Option<DateTime<Utc>>,
    result: String,
    log: LogBuffer,
}

impl Job {
    pub fn new(job_submission: JobSubmission) -> Self {
        let now = Utc::now();
        let this = Self {
            id: Ulid::new(),
            submission: job_submission,
            state: State::INIT,
            created_at: now,
            started_at: None,
            finished_at: None,
            result: String::new(),
            log: LogBuffer::new(),
        };
        println!("[Job]: new: job {} created at {}", this.id, this.created_at);
        this
    }
}

/**
 * JobCell
 * Shared, thread safe job
 */
pub enum JobCell {
    NotCompleted(Arc<Mutex<Job>>),
    Completed,
}

/**
 * JobPoolState
 * Fixed-length (for now) set of jobs
 */
struct JobPoolState {
    // fixed size vec of active jobs
    max_len: usize,
    jobs: Vec<Option<JobCell>>,
    // completed jobs
    completed: Vec<Job>,
}

impl JobPoolState {
    pub fn new() -> Self {
        let max_len: usize = 4;
        Self {
            jobs: Vec::new(),
            max_len,
            completed: Vec::new(),
        }
    }

    // Find a slot in which to queue a new job
    pub fn find_slot(&mut self) -> Option<usize> {
        // if we don't yet have max_len, we know we have room
        if self.jobs.len() < self.max_len {
            self.jobs.push(None);
            return Some(self.jobs.len() - 1);
        }
        // iterate and find an open slot
        // NOTE: O(n) operation is not optimal (optimize later)
        for (i, opt) in self.jobs.iter().enumerate() {
            match opt {
                Some(cell) => match cell {
                    JobCell::NotCompleted(_) => continue,
                    JobCell::Completed => return Some(i),
                },
                None => continue,
            }
        }
        // job pool is full and none are completed: no room
        None
    }

    // Fail a new job
    // NOTE: Takes ownership of job
    fn fail_new_job(&mut self, mut job: Job, reason: &str) {
        println!("[jobpoolstate]: job {} failed ({})", job.id, reason);
        job.state = State::FAILED;
        job.result = reason.to_string();
        self.completed.push(job);
    }

    // Queue a new job
    // NOTE: Takes ownership of job
    fn queue_new_job(&mut self, mut job: Job, index: usize) {
        debug_assert!(index < self.jobs.len());
        job.state = State::QUEUED;
        job.log.logf(
            LogLevel::INFO,
            format_args!("queued at {}", chrono::Utc::now()),
        );
        let cell = JobCell::NotCompleted(std::sync::Arc::new(Mutex::new(job)));
        self.jobs[index] = Some(cell);
    }

    // Handle a new job submission
    fn handle_new_job(&mut self, job_submission: JobSubmission) {
        println!(
            "handle_new_job: received job submission {:?}",
            job_submission
        );
        // create the job
        let newjob = Job::new(job_submission);
        // if we have room, queue it; otherwise fail
        match self.find_slot() {
            None => self.fail_new_job(newjob, "pool full: job never queued"),
            Some(i) => self.queue_new_job(newjob, i),
        }
    }
}

/**
 * JobPool
 * High-level job pool data structure
 * Encapsulates shared, thread-safe access to the pool state
 * Runs an async loop that handles new job submissions
 */
pub struct JobPool {
    pool: Arc<Mutex<JobPoolState>>,
    submission_tx: mpsc::Sender<JobSubmission>,
}

impl JobPool {
    pub fn start() -> Arc<Self> {
        // message-passing channel
        // TX used by API to submit new jobs
        // RX used in job loop to receive requests and handle new job submissions
        println!("JobPool: start");
        println!("JobPool: create job submisssion channel");
        let (submission_tx, mut submission_rx) = mpsc::channel(32);
        let state = JobPoolState::new();
        let pool = Arc::new(Mutex::new(state));

        // "this" is an Arc wrapped pool
        // private constructor pattern
        println!("JobPool: create new pool");
        let this = Arc::new(Self {
            pool: pool.clone(),
            submission_tx,
        });

        // Get a new reference to the pool and provide it to the pool execution loop
        println!("JobPool: spawning run loop");
        let pool_clone = pool.clone();
        tokio::spawn(async move {
            JobPool::run_loop(pool_clone, &mut submission_rx).await;
        });

        // Return "this" so calling function has the pool
        this
    }

    async fn run_loop(pool: Arc<Mutex<JobPoolState>>, rx: &mut mpsc::Receiver<JobSubmission>) {
        println!("JobPool: [run_loop]: starting");
        while let Some(job) = rx.recv().await {
            println!("JobPool: [run_loop]: received job {:?}", job);
            // acquire lock
            let mut p = pool.lock().await;
            p.handle_new_job(job);
            println!("JobPool: [run_loop]: job {:?} handling complete", job);
            // release lock
            drop(p);
        }
    }

    /**
     * submit: submit a job to the pool
     */
    pub async fn submit(&self, job: JobSubmission) {
        self.submission_tx.send(job).await.unwrap();
    }
}
