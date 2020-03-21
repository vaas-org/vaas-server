use actix::prelude::*;
use actix_web::{web, App, Error, HttpRequest, HttpResponse, HttpServer};
use actix_web_actors::ws;
use slog::info;

mod log;
mod websocket;
mod services;

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
    let service_addr = services::Service::new(logger.clone()).start();

    info!(logger, "Starting WS server");

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
