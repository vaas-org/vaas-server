use actix::prelude::*;
use actix::registry::Registry;
use actix::registry::SystemRegistry;
use actix_web::{web, Error, HttpRequest, HttpResponse};
use actix_web_actors::ws;
use services::broadcast::BroadcastActor;
use services::client::ClientActor;
use services::issue::IssueService;
use services::{session::SessionActor, vote::VoteActor};
use tracing::{info, span, Level};

use crate::{db, services, websocket};
use sqlx::PgPool;

async fn ws_route(req: HttpRequest, stream: web::Payload) -> Result<HttpResponse, Error> {
    let span = span!(Level::INFO, "ws_route");
    let _enter = span.enter();
    register_request_actors();
    let ws_client = websocket::WsClient::new();
    ws::start(ws_client, &req, stream)
}

pub fn register_db_actor(pool: PgPool) {
    let db_executor = db::DbExecutor(pool);
    SystemRegistry::set(db_executor.start());
}

pub fn register_request_actors() {
    info!("Registering request actors");
}

pub fn register_arbiter_actors() {
    info!("Registering arbiter actors");
    Registry::set(IssueService::mocked().start());
    Registry::set(services::Service::new().start());
}

pub fn register_system_actors() {
    info!("Registering system actors");
    SystemRegistry::set(VoteActor::new().start());
    SystemRegistry::set(BroadcastActor::new().start());
    SystemRegistry::set(ClientActor::new().start());
    SystemRegistry::set(SessionActor::default().start());
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    let span = span!(Level::INFO, "my_span");
    let _enter = span.enter();
    register_arbiter_actors();
    // websocket
    cfg.service(web::resource("/ws/").to(ws_route));
}
