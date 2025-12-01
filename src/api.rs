/*! API module for async job orchestrator */
use std::sync::Arc;

use crate::jobs::JobPool;
use crate::jobs::JobSubmission;
use axum::{Json, Router, extract::State as AxumState, routing::get, routing::post};

/**
Creates the main application router and wires up all the handlers.
Takes a job pool Arc as the API state
*/
pub fn create_router(pool: Arc<JobPool>) -> Router {
    // This `app` router is private to the `api` module.
    // We are encapsulating the routing logic here.
    Router::new()
        .route("/jobs", post(post_jobs).get(get_jobs))
        .route("/metrics", get(get_metrics))
        .with_state(pool)
}

/**
Submit a new job for immediate execution
*/
async fn post_jobs(AxumState(pool): AxumState<Arc<JobPool>>, Json(req): Json<JobSubmission>) {
    println!("[api] Job submitted: {:?}", req);
    pool.submit(req).await;
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
