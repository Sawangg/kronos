use crate::routes::run::run;
use axum::{routing::post, Router};

mod broker;
mod data;
mod engine;
mod routes;
mod strategy;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    let app = Router::new().route("/run", post(run));

    let port = std::env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    println!("Listening on port {}", port);

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port))
        .await
        .unwrap();
    axum::serve(listener, app).await.unwrap();
}
