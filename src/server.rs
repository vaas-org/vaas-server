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
) -> Result<HttpResponse, Error> {
    let ws_client = websocket::WsClient::new(service_addr.get_ref().clone());
    ws::start(ws_client, &req, stream)
}

pub fn register_system_actors() {
    // register actors with logger
    SystemRegistry::set(VoteActor::new().start());
    SystemRegistry::set(BroadcastActor::new().start());
    SystemRegistry::set(ClientActor::new().start());
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    let service_addr = services::Service::new().start();
    // websocket
    cfg.data(service_addr)
        .service(web::resource("/ws/").to(ws_route));
}
