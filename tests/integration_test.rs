extern crate vaas_server;
use actix_web::{test, App};
use actix_web_actors::ws;
use futures::{SinkExt, StreamExt};
use vaas_server::{log, server, websocket};
use websocket::{IncomingMessage, IncomingVote, OutgoingMessage};

macro_rules! frame_message_type {
    ($framed:expr, $message_type:path) => {
        match $framed.next().await.unwrap().unwrap() {
            ws::Frame::Text(item) => {
                let item: OutgoingMessage = serde_json::from_slice(&item[..]).unwrap();
                match item {
                    $message_type(message_type) => message_type,
                    _ => panic!("Wrong outgoing message type"),
                }
            }
            _ => panic!("Websocket frame should be text"),
        }
    };
}

#[actix_rt::test]
async fn test_vote() {
    // Setup test server
    let mut srv = test::start(|| {
        let logger = log::logger();
        server::register_system_actors(logger.clone());
        App::new().configure(|app| server::configure(app, logger.clone()))
    });
    let mut framed = srv.ws_at("/ws/").await.unwrap();

    // Send vote
    let message = IncomingMessage::Vote(IncomingVote {
        user_id: "123".to_string(),
        alternative_id: "1".to_string(),
    });
    let message = serde_json::to_string(&message).unwrap();
    framed.send(ws::Message::Text(message)).await.unwrap();

    let client = frame_message_type!(framed, OutgoingMessage::Client);
    assert_eq!(client.username, None);

    let issue = frame_message_type!(framed, OutgoingMessage::Issue);
    assert_eq!(issue.title, "coronvorus bad??");

    let vote = frame_message_type!(framed, OutgoingMessage::Vote);
    assert_eq!(vote.alternative_id, "1");
    assert_eq!(vote.user_id, "123");

    // Close connection
    framed
        .send(ws::Message::Close(Some(ws::CloseCode::Normal.into())))
        .await
        .unwrap();

    let item = framed.next().await.unwrap().unwrap();
    assert_eq!(item, ws::Frame::Close(Some(ws::CloseCode::Normal.into())));
}
