use actix::prelude::*;
use actix_web::{web, App, Error, HttpRequest, HttpResponse, HttpServer};
use actix_web_actors::ws;

mod service;
mod websocket;

async fn ws_route(
    req: HttpRequest,
    stream: web::Payload,
    service_addr: web::Data<Addr<service::Service>>,
) -> Result<HttpResponse, Error> {
    ws::start(
        websocket::WsClient {
            id: 0,
            service: service_addr.get_ref().clone(),
        },
        &req,
        stream,
    )
}

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    let service_addr = service::Service::default().start();

    // Create Http server with websocket support
    HttpServer::new(move || {
        App::new()
            // websocket
            .data(service_addr.clone())
            .service(web::resource("/ws/").to(ws_route))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
