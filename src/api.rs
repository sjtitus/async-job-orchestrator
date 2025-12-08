/*! API module for async job orchestrator */
use axum::{
    Json, Router, extract::State as AxumState, http::StatusCode, routing::get, routing::post,
};
use std::sync::Arc;

use crate::api_error::ApiError;
use crate::jobs::{Job, JobPool, JobSubmission};

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
async fn post_jobs(
    AxumState(pool): AxumState<Arc<JobPool>>,
    Json(req): Json<JobSubmission>,
) -> Result<StatusCode, ApiError> {
    println!("[api] Job submitted: {:?}", req);
    pool.submit(req).await?;
    Ok(StatusCode::ACCEPTED)
}

/**
Get the active jobs
*/
async fn get_jobs(
    AxumState(pool): AxumState<Arc<JobPool>>,
) -> Result<(StatusCode, Json<Vec<Job>>), ApiError> {
    let jobs = pool.get_jobs().await?;
    Ok((StatusCode::OK, Json(jobs)))
}

/**
Get job orchestrator metrics
*/
async fn get_metrics() -> &'static str {
    println!("GET metrics");
    return "get /metrics";
}
