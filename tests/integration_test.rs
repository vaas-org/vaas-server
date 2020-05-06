extern crate vaas_server;
use actix_codec::Framed;
use actix_http::ws::Codec;
use actix_web::{test, App};
use actix_web_actors::ws;
use futures::{SinkExt, StreamExt};
use insta::assert_ron_snapshot;
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::time::timeout;
use vaas_server::{log, server, websocket};
use websocket::{IncomingMessage, IncomingVote, OutgoingMessage};

const READ_TIMEOUT_MS: u64 = 200;

macro_rules! frame_message_type {
    ($framed:expr, $message_type:path) => {
        match read_message(&mut $framed)
            .await
            .expect("Unable to read ws frame")
        {
            $message_type(message_type) => message_type,
            _ => panic!("Wrong outgoing message type"),
        }
    };
}

async fn read_message(
    framed: &mut Framed<impl AsyncRead + AsyncWrite, Codec>,
) -> Option<OutgoingMessage> {
    let frame = timeout(Duration::from_millis(READ_TIMEOUT_MS), framed.next()).await;
    // ???
    match frame.ok()??.unwrap() {
        ws::Frame::Text(item) => Some(serde_json::from_slice(&item[..]).unwrap()),
        _ => None,
    }
}

async fn read_messages(
    mut framed: &mut Framed<impl AsyncRead + AsyncWrite, Codec>,
) -> Vec<OutgoingMessage> {
    let mut messages = vec![];
    while let Some(message) = read_message(&mut framed).await {
        messages.push(message);
    }
    messages
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

    let item = timeout(Duration::from_millis(READ_TIMEOUT_MS), framed.next())
        .await
        .expect("timeout")
        .unwrap()
        .unwrap();
    assert_eq!(item, ws::Frame::Close(Some(ws::CloseCode::Normal.into())));
}

#[actix_rt::test]
async fn test_connect() {
    // Setup test server
    let mut srv = test::start(|| {
        let logger = log::logger();
        server::register_system_actors(logger.clone());
        App::new().configure(|app| server::configure(app, logger.clone()))
    });
    let mut framed = srv.ws_at("/ws/").await.unwrap();

    let messages = read_messages(&mut framed).await;

    assert_ron_snapshot!(messages, { ".**.id" => "[uuid]" });
}
