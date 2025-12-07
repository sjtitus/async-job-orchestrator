/*! Jobs module for async orchestrator
 * Defines job structures
 */
use crate::api_error::ApiError;
use crate::logs::{LogBuffer, LogLevel};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::sync::Arc;
use tokio::sync::{
    Mutex,
    mpsc::{self},
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

impl fmt::Display for State {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            State::INIT => "init",
            State::QUEUED => "queued",
            State::RUNNING => "running",
            State::SUCCEEDED => "succeeded",
            State::FAILED => "failed",
        };
        f.write_str(s)
    }
}

/**
 * Job payloads
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
 * Submitted by API
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
 */
#[derive(Clone, Debug)]
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

impl fmt::Display for Job {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "\nId: {}\nState: {}\nCreated: {}\nResult: {}\nLogs:\n{}",
            self.id, self.state, self.created_at, self.result, self.log,
        )
    }
}

impl Job {
    pub fn new(job_submission: &JobSubmission) -> Self {
        let now = Utc::now();
        let this = Self {
            id: Ulid::new(),
            submission: job_submission.clone(),
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
 * Contains a shared, thread safe job
 * Empty: cell is free to use
 */
#[derive(Clone)]
pub enum JobCell {
    Empty,
    Occupied(Arc<std::sync::Mutex<Job>>),
}

/**
 * JobPoolState
 * Fixed-length (for now) set of max_jobs jobs
 * NOTE: Option None --> job is being executed in a another thread
 */
struct JobPoolState {
    jobs: Vec<Option<JobCell>>,
    max_jobs: usize,
    completed: Vec<Job>,
}

impl JobPoolState {
    // new: create sized job pool
    pub fn new(max_jobs: usize) -> Self {
        debug_assert!(max_jobs > 0);
        Self {
            max_jobs,
            jobs: Vec::new(),
            completed: Vec::new(),
        }
    }

    // Find a slot for a new job
    // Returns index of slot or None on full
    pub fn find_slot(&mut self) -> Option<usize> {
        // slots left: create a new empty slot
        if self.jobs.len() < self.max_jobs {
            self.jobs.push(Some(JobCell::Empty));
            return Some(self.jobs.len() - 1);
        }
        // search for an open slot
        // TODO: optimize to eliminate this O(n) operation
        for (i, opt) in self.jobs.iter().enumerate() {
            match opt {
                Some(cell) => match cell {
                    JobCell::Occupied(_) => continue,
                    JobCell::Empty => return Some(i),
                },
                // None: job is being executed
                None => continue,
            }
        }
        // Full
        None
    }

    // Fail a job
    // NOTE: takes ownership of job
    fn fail_and_complete_job(&mut self, mut job: Job, reason: &str) {
        job.state = State::FAILED;
        job.result = reason.to_string();
        self.completed.push(job);
    }

    // Run a job
    // NOTE: takes ownership of job
    fn run_job(&mut self, mut job: Job, index: usize, completion_tx: &mpsc::Sender<usize>) {
        debug_assert!(index < self.jobs.len());
        debug_assert!(matches!(self.jobs[index], Some(JobCell::Empty)));

        job.state = State::QUEUED;
        job.log.logf(
            LogLevel::INFO,
            format_args!("queued at {}", chrono::Utc::now()),
        );

        let cell = JobCell::Occupied(Arc::new(std::sync::Mutex::new(job)));
        self.jobs[index] = Some(cell);
        // TAKE the job out immediately
        let cell = self.jobs[index].take().expect("job just inserted");

        let completion_tx = completion_tx.clone();
        tokio::task::spawn_blocking(move || {
            JobPoolState::run_job_blocking(cell, index, completion_tx);
        });
    }

    fn run_job_blocking(cell: JobCell, index: usize, completion_tx: mpsc::Sender<usize>) {
        let JobCell::Occupied(job_arc) = cell else {
            panic!("run_job_blocking called with non-occupied cell");
        };

        let job_submission: JobSubmission;

        {
            let mut job = job_arc.lock().unwrap();
            job.state = State::RUNNING;
            job.log.logf(LogLevel::INFO, format_args!("job started"));
            job_submission = job.submission.clone();
        }

        // === ACTUAL WORK HERE ===
        // do heavy computation / I/O / blocking call
        println!("[JobPoolState]: ===========================");
        println!("[JobPoolState]: RUNNING JOB\n{:#?}", job_submission);
        println!("[JobPoolState]: ===========================");

        {
            let mut job = job_arc.lock().unwrap();
            job.state = State::SUCCEEDED;
            job.log.logf(LogLevel::INFO, format_args!("job finished"));
        }

        completion_tx.blocking_send(index).unwrap();
    }

    // Handle a job submission
    fn handle_new_job(
        &mut self,
        job_submission: &JobSubmission,
        completion_tx: &mpsc::Sender<usize>,
    ) {
        // Create the job
        // if we have room, queue it; otherwise fail
        let mut newjob = Job::new(job_submission);
        println!("[JobPoolState]: job {}: created", newjob.id);
        match self.find_slot() {
            None => {
                println!("[JobPoolState]: job {}: failed (pool full)", newjob.id);
                self.fail_and_complete_job(newjob, "pool full: job never queued");
            }
            Some(i) => {
                println!("[JobPoolState]: queueing job {}: index {}", newjob.id, i);
                self.run_job(newjob, i, completion_tx);
            }
        }
    }

    fn finish_job(&mut self, job_index: usize) {
        println!("[JobPoolState]: job {}: finishing", job_index);
    }
}

/**
 * JobPool
 */
pub struct JobPool {
    pool: Arc<Mutex<JobPoolState>>,
    // used by API to submit jobs to the pool
    submission_tx: mpsc::Sender<JobSubmission>,
}

impl JobPool {
    pub fn start() -> Arc<Self> {
        println!("[JobPool]: start");

        // message-passing channels
        println!("[JobPool]: creating job messaging channels");
        // channel for job submissions
        let (submission_tx, mut submission_rx) = mpsc::channel(32);
        // channel for job completions
        let (completion_tx, mut completion_rx) = mpsc::channel::<usize>(32);

        // construct underlying pool state
        println!("[JobPool]: create new pool");
        let state = JobPoolState::new(4);
        let pool = Arc::new(Mutex::new(state));
        // NOTE: private constructor pattern
        let this = Arc::new(Self {
            pool: pool.clone(),
            submission_tx,
        });

        // Spawn the async loop that handles job submissions and completions
        println!("[JobPool]: spawning job handling loop");
        let pool_clone = pool.clone();
        tokio::spawn(async move {
            JobPool::run_loop(
                pool_clone,
                // receives submissions
                &mut submission_rx,
                // receives completions
                &mut completion_rx,
                // provides completion channel to execution threads
                completion_tx,
            )
            .await;
        });

        // private constructor pattern:
        // return "this" so calling function has the pool
        this
    }

    async fn run_loop(
        pool: Arc<Mutex<JobPoolState>>,
        submission_rx: &mut mpsc::Receiver<JobSubmission>,
        completion_rx: &mut mpsc::Receiver<usize>,
        completion_tx: mpsc::Sender<usize>,
    ) {
        println!("[JobPool]: [run_loop]: starting");
        loop {
            tokio::select! {

                // ----------------------------------------
                // New job submitted
                // ----------------------------------------
                Some(job_submission) = submission_rx.recv() => {
                    println!("[JobPool]: [run_loop]: job submission received: {:?}", job_submission);
                    // acquire lock
                    let mut p = pool.lock().await;
                    let completion_tx_channel = completion_tx.clone();
                    p.handle_new_job(&job_submission, &completion_tx_channel);
                    println!("[JobPool]: [run_loop]: job submission complete: {:?}", job_submission);
                    // release lock
                    drop(p);
                }

                // ----------------------------------------
                // Job completed
                // ----------------------------------------
                Some(completed_job_index) = completion_rx.recv() => {
                    println!("[JobPool]: [run_loop]: job completion received: {}", completed_job_index);
                    // acquire lock
                    let mut p = pool.lock().await;
                    p.finish_job(completed_job_index);
                    // release lock
                    println!("[JobPool]: [run_loop]: job completion processed: {}", completed_job_index);
                    drop(p);
                }
            }
        }
    }

    /**
     * submit: submit a job to the pool
     */
    pub async fn submit(&self, job: JobSubmission) -> Result<(), ApiError> {
        self.submission_tx
            .send(job)
            .await
            .map_err(|_| ApiError::JobQueueClosed)
    }
}
