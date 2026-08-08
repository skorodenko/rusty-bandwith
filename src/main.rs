mod cli;
mod handlers;
mod util;

use axum::{routing::get, Router};
use clap::Parser;
use reqwest::Client;
use std::net::SocketAddr;
use std::sync::Arc;
use tracing;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Parse command line arguments
    let args = cli::Args::parse();

    // Create shared configuration
    let config = Arc::new(cli::AppConfig {
        mp_cap: args.mp_cap,
    });

    // Set up the server to listen on localhost with the specified port
    let addr = SocketAddr::new(args.host, args.port);

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_writer(std::io::stdout)
        .init();

    // Create a service that will handle incoming requests
    let config_clone = config.clone();
    let client = Client::new();
    let app = Router::new()
        .route("/", get(handlers::root_handler))
        .route("/{*path}", get(handlers::proxy_handler))
        .with_state(config_clone)
        .with_state(client);

    // Start the server
    match tokio::net::TcpListener::bind(addr).await {
        Ok(listener) => {
            tracing::info!("Listening on http://{}", addr);
            axum::serve(listener, app).await.unwrap();
        }
        Err(e) => {
            tracing::error!("Failed to start server on {}: {}", addr, e);
        }
    }

    Ok(())
}
