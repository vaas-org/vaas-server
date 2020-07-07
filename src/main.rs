#![warn(clippy::all)]
use actix_web::{App, HttpServer};
use dotenv::dotenv;
use tracing::{error, info, Level};
use tracing_error::ErrorLayer;
use tracing_subscriber::prelude::*;
extern crate vaas_server;

use std::env;
use vaas_server::{db, server};

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    let database_url = env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set. e.g: postgres://postgres:postgres@localhost");
    let pool = db::new_pool(&database_url).await.unwrap();
    server::register_db_actor(pool);
    server::register_system_actors();

    // Global tracing subscriber
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(Level::DEBUG)
        .finish()
        .with(ErrorLayer::default());
    tracing::subscriber::set_global_default(subscriber).unwrap();

    if let Err(err) = color_eyre::install() {
        error!("Failed to install eyre {:#?}", err);
    }

    info!("Starting server");

    // Create Http server with websocket support
    HttpServer::new(move || App::new().configure(|app| server::configure(app)))
        .bind("127.0.0.1:8080")?
        .run()
        .await
}
