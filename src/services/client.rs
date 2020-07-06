use super::Login;
use crate::span::{SpanHandler, SpanMessage};
use crate::{
    db::{
        user::{UserByUsername, UserId},
        DbExecutor,
    },
    message_handler_with_span,
    websocket::WsClient,
};
use actix::prelude::*;
use actix_interop::FutureInterop;
use color_eyre::eyre::WrapErr;
use std::collections::HashMap;
use std::collections::HashSet;
use tracing::{debug, info, Span};

// Types

// Actor
pub struct ClientActor {
    clients: HashMap<UserId, HashSet<Addr<WsClient>>>,
}

impl ClientActor {
    pub fn new() -> Self {
        ClientActor {
            clients: HashMap::new(),
        }
    }
}

impl Default for ClientActor {
    fn default() -> Self {
        unimplemented!("Client actor can't be unitialized using default because it needs a logger")
    }
}

impl Actor for ClientActor {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        info!("Client actor started");
    }
}

message_handler_with_span! {
    impl SpanHandler<Login> for ClientActor {
        type Result = ResponseActFuture<Self, <Login as Message>::Result>;

        fn handle(&mut self, msg: Login, _ctx: &mut Context<Self>, span: Span) -> Self::Result {
            debug!("Incoming login in ClientActor");
            async {
                DbExecutor::from_registry()
                    .send(SpanMessage::new(UserByUsername(msg.username), span))
                    .await
                    .wrap_err("Failed to get user by username")?
            }.interop_actor_boxed(self)
        }
    }
}

impl SystemService for ClientActor {}
impl Supervised for ClientActor {}
