mod api;
mod jobs;
mod logs;

#[tokio::main]
async fn main() {
    println!("[main] Starting application");

    // Create the router from the `api` module
    let app = api::create_router();

    let addr = "0.0.0.0:3000";
    println!("[main] Serving on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();

    // Run the app
    axum::serve(listener, app).await.unwrap();
}
