mod api;
mod jobs;
mod logs;

use jobs::JobPool;

#[tokio::main]
async fn main() {
    println!("[main] Starting application");

    println!("[main] Starting jobpool");
    let job_pool = JobPool::start();

    // Create the router that the API will use
    // Embed the job pool as app specific data
    println!("[main] Creating router");
    let app = api::create_router(job_pool.clone());

    let addr = "0.0.0.0:3000";
    println!("[main] Serving on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();

    // Run the app
    axum::serve(listener, app).await.unwrap();
}
