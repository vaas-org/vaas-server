extern crate vaas_server;
use actix_codec::Framed;
use actix_http::ws::Codec;
use actix_web::{test, App};
use actix_web_actors::ws;
use db::alternative::AlternativeId;
use dotenv::dotenv;
use futures::{SinkExt, StreamExt};
use insta::assert_ron_snapshot;
use sqlx::PgPool;
use std::env;
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::time::timeout;
use vaas_server::{
    db::{self, user::UserId},
    server, websocket,
};
use websocket::{IncomingLogin, IncomingMessage, IncomingReconnect, IncomingVote, OutgoingMessage};

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

async fn pg_pool() -> PgPool {
    // TODO: read from some shared config
    dotenv().ok();
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL not set");
    db::new_pool(&database_url).await.unwrap()
}

#[actix_rt::test]
async fn test_login_user() {
    // Setup test serve
    let pool = pg_pool().await;
    let mut srv = test::start(move || {
        server::register_db_actor(pool.clone());
        server::register_system_actors();
        App::new().configure(|app| server::configure(app))
    });
    let mut framed = srv.ws_at("/ws/").await.unwrap();
    // Wait until issue has been sent
    frame_message_type!(framed, OutgoingMessage::Issue);

    // Send user login
    let message = IncomingMessage::Login(IncomingLogin {
        username: "user".to_owned(),
    });
    let message = serde_json::to_string(&message).unwrap();
    framed.send(ws::Message::Text(message)).await.unwrap();

    let messages = read_messages(&mut framed).await;
    assert_ron_snapshot!(messages, { ".**.id" => "[uuid]" });
}

#[actix_rt::test]
async fn test_reconnect() {
    // Setup test server
    let pool = pg_pool().await;
    let mut srv = test::start(move || {
        server::register_db_actor(pool.clone());
        server::register_system_actors();
        App::new().configure(|app| server::configure(app))
    });
    let mut framed = srv.ws_at("/ws/").await.unwrap();
    // Wait until issue has been sent
    frame_message_type!(framed, OutgoingMessage::Issue);

    // Send user login
    let message = IncomingMessage::Login(IncomingLogin {
        username: "user".to_owned(),
    });
    let message = serde_json::to_string(&message).unwrap();
    framed.send(ws::Message::Text(message)).await.unwrap();

    let messages = read_messages(&mut framed).await;
    assert_ron_snapshot!("reconnect - initial login", messages, { ".**.id" => "[uuid]" });

    // TODO: rewrite to filter?
    let mut session_id = None;
    for message in messages {
        match message {
            OutgoingMessage::Client(client) => {
                session_id = Some(client.id);
                break;
            }
            _ => continue,
        }
    }

    let mut framed = srv.ws_at("/ws/").await.unwrap();
    // Wait until issue has been sent
    frame_message_type!(framed, OutgoingMessage::Issue);
    let message = IncomingMessage::Reconnect(IncomingReconnect {
        session_id: session_id.expect("Session id should exist"),
    });
    let message = serde_json::to_string(&message).unwrap();
    framed.send(ws::Message::Text(message)).await.unwrap();

    let messages = read_messages(&mut framed).await;
    assert_ron_snapshot!("reconnect - reload", messages, { ".**.id" => "[uuid]" });
}

#[actix_rt::test]
async fn test_vote() {
    // Setup test server
    let pool = pg_pool().await;
    let mut srv = test::start(move || {
        server::register_db_actor(pool.clone());
        server::register_system_actors();
        App::new().configure(|app| server::configure(app))
    });

    let mut framed = srv.ws_at("/ws/").await.unwrap();
    let issue = frame_message_type!(framed, OutgoingMessage::Issue);
    assert_eq!(issue.title, "coronvorus bad??");

    let user_id = UserId::new();
    let alternative_id = AlternativeId::new();

    // Send vote
    let message = IncomingMessage::Vote(IncomingVote {
        user_id: Some(user_id.clone()),
        alternative_id: alternative_id.clone(),
    });
    let message = serde_json::to_string(&message).unwrap();
    framed.send(ws::Message::Text(message)).await.unwrap();

    // let client = frame_message_type!(framed, OutgoingMessage::Client);
    // assert_eq!(client.username, None);

    let vote = frame_message_type!(framed, OutgoingMessage::Vote);
    assert_eq!(vote.alternative_id, alternative_id);
    assert_eq!(vote.user_id, user_id);

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
    let pool = pg_pool().await;
    let mut srv = test::start(move || {
        server::register_db_actor(pool.clone());
        server::register_system_actors();
        App::new().configure(|app| server::configure(app))
    });
    let mut framed = srv.ws_at("/ws/").await.unwrap();

    let messages = read_messages(&mut framed).await;

    assert_ron_snapshot!(messages, { ".**.id" => "[uuid]" });
}
