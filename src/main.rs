use actix::prelude::*;
use actix::registry::SystemRegistry;
use actix_web::{web, App, Error, HttpRequest, HttpResponse, HttpServer};
use actix_web_actors::ws;
use services::broadcast::BroadcastActor;
use services::vote::VoteActor;
use services::user::UserActor;
use slog::info;

mod log;
mod services;
mod websocket;

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

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    let logger = log::logger();

    info!(logger, "Starting WS server");

    // register actors with logger
    SystemRegistry::set(VoteActor::new(logger.clone()).start());
    SystemRegistry::set(BroadcastActor::new(logger.clone()).start());
    SystemRegistry::set(UserActor::new(logger.clone()).start());

    let service_addr = services::Service::new(logger.clone()).start();
    // Create Http server with websocket support
    HttpServer::new(move || {
        App::new()
            // websocket
            .data(service_addr.clone())
            .data(logger.clone())
            .service(web::resource("/ws/").to(ws_route))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
