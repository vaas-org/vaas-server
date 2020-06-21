#![warn(clippy::all)]
use actix_web::{App, HttpServer};
use tracing::{info, Level};
extern crate vaas_server;

use vaas_server::server;

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    server::register_system_actors();

    // Global tracing subscriber
    tracing_subscriber::fmt()
        .with_max_level(Level::DEBUG)
        .init();

    info!("Starting server");

    // Create Http server with websocket support
    HttpServer::new(move || App::new().configure(|app| server::configure(app)))
        .bind("127.0.0.1:8080")?
        .run()
        .await
}
