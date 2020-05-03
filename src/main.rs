#![warn(clippy::all)]
use actix_web::{App, HttpServer};
use slog::info;
extern crate vaas_server;

use vaas_server::{log, server};

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    let logger = log::logger();

    info!(logger, "Starting server");

    server::register_system_actors(logger.clone());

    // Create Http server with websocket support
    HttpServer::new(move || App::new().configure(|app| server::configure(app, logger.clone())))
        .bind("127.0.0.1:8080")?
        .run()
        .await
}
