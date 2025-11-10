/*! API module for async job orchestrator */
use crate::jobs::{JobSubmission, JobSubmissionResponse, State};
use axum::{Json, Router, routing::get, routing::post};

/**
Creates the main application router and wires up all the handlers.
This is the public entry point for this module.
*/
pub fn create_router() -> Router {
    // This `app` router is private to the `api` module.
    // We are encapsulating the routing logic here.
    Router::new()
        .route("/jobs", post(post_jobs).get(get_jobs))
        .route("/metrics", get(get_metrics))
}

/**
Submit a new job for immediate execution
*/
async fn post_jobs(Json(req): Json<JobSubmission>) -> Json<JobSubmissionResponse> {
    println!("[api] Job submitted: {:?}", req);
    let resp = JobSubmissionResponse::new(req, State::QUEUED);
    Json(resp)
}

/**
Get the status/result of a submitted job
*/
async fn get_jobs() -> &'static str {
    println!("GET jobs");
    return "get /jobs";
}

/**
Get job orchestrator metrics
*/
async fn get_metrics() -> &'static str {
    println!("GET metrics");
    return "get /metrics";
}
