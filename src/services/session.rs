use crate::async_message_handler_with_span;
use crate::{
    db::{
        self,
        session::{InternalSession, SessionId},
        DbExecutor,
    },
    span::{AsyncSpanHandler, SpanMessage},
};
use actix::prelude::*;
use color_eyre::eyre::Report;
use db::user::UserId;
use tracing::info;
use tracing::{debug, span, Level};

#[derive(Default)]
pub struct SessionActor {}

impl Actor for SessionActor {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        info!("Session actor started");
    }
}

impl SystemService for SessionActor {}
impl Supervised for SessionActor {}

#[derive(Message, Clone)]
#[rtype(result = "Result<Option<InternalSession>, Report>")]
pub struct SessionById(pub SessionId);

async_message_handler_with_span!({
    impl AsyncSpanHandler<SessionById> for SessionActor {
        async fn handle(msg: SessionById) -> Result<Option<InternalSession>, Report> {
            debug!("Handling session by id");
            DbExecutor::from_registry()
                .send(SpanMessage::new(db::session::SessionById(msg.0)))
                .await?
        }
    }
});

#[derive(Message, Clone)]
#[rtype(result = "Result<InternalSession, Report>")]
pub struct SaveSession(pub UserId);

async_message_handler_with_span!({
    impl AsyncSpanHandler<SaveSession> for SessionActor {
        async fn handle(msg: SaveSession) -> Result<InternalSession, Report> {
            let user_id = msg.0.clone();
            let span = span!(
                Level::DEBUG,
                "save session",
                user_id = user_id.as_string().as_str()
            );
            let _enter = span.enter();
            debug!("Saving session");
            DbExecutor::from_registry()
                .send(SpanMessage::new(db::session::SaveSession(msg.0)))
                .await?
        }
    }
});
