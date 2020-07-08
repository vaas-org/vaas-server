use super::Login;
use crate::span::{AsyncSpanHandler, SpanMessage};
use crate::{
    async_message_handler_with_span,
    db::{user::UserByUsername, DbExecutor},
};
use actix::prelude::*;
use color_eyre::{eyre::WrapErr, Report};
use tracing::{debug, info};

// Types

// Actor
pub struct ClientActor {}

impl ClientActor {
    pub fn new() -> Self {
        ClientActor {}
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

async_message_handler_with_span!({
    impl AsyncSpanHandler<Login> for ClientActor {
        async fn handle(msg: Login) -> Result<Option<crate::db::user::InternalUser>, Report> {
            debug!("Incoming login in ClientActor");
            DbExecutor::from_registry()
                .send(SpanMessage::new(UserByUsername(msg.username)))
                .await
                .wrap_err("Failed to get user by username")?
        }
    }
});

impl SystemService for ClientActor {}
impl Supervised for ClientActor {}
