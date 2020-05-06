use actix::prelude::*;
use actix::registry::SystemRegistry;
use actix_web::{web, Error, HttpRequest, HttpResponse};
use actix_web_actors::ws;
use services::broadcast::BroadcastActor;
use services::client::ClientActor;
use services::vote::VoteActor;

use crate::{services, websocket};

async fn ws_route(
    req: HttpRequest,
    stream: web::Payload,
    service_addr: web::Data<Addr<services::Service>>,
    logger: web::Data<slog::Logger>,
) -> Result<HttpResponse, Error> {
    ws::start(
        websocket::WsClient::new(service_addr.get_ref().clone(), logger.get_ref().clone()),
        &req,
        stream,
    )
}

pub fn register_system_actors(logger: slog::Logger) {
    // register actors with logger
    SystemRegistry::set(VoteActor::new(logger.clone()).start());
    SystemRegistry::set(BroadcastActor::new(logger.clone()).start());
    SystemRegistry::set(ClientActor::new(logger.clone()).start());
}

pub fn configure(cfg: &mut web::ServiceConfig, logger: slog::Logger) {
    let service_addr = services::Service::new(logger.clone()).start();
    // websocket
    cfg.data(service_addr)
        .data(logger.clone())
        .service(web::resource("/ws/").to(ws_route));
}
