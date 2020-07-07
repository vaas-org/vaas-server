use crate::message_handler_with_span;
use crate::{
    db::{
        self,
        session::{InternalSession, SessionId},
        DbExecutor,
    },
    span::{SpanHandler, SpanMessage},
};
use actix::prelude::*;
use actix_interop::FutureInterop;
use color_eyre::eyre::Report;
use db::user::UserId;
use tracing::info;
use tracing::{debug, span, Level, Span};

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

message_handler_with_span! {
    impl SpanHandler<SessionById> for SessionActor {
        type Result = ResponseActFuture<Self, <SessionById as Message>::Result>;

        fn handle(&mut self, msg: SessionById, _ctx: &mut Context<Self>, span: Span) -> Self::Result {
            async {
                debug!("Handling session by id");
                DbExecutor::from_registry().send(SpanMessage::new(db::session::SessionById(msg.0))).await?
            }.interop_actor_boxed(self)
        }
    }
}
#[derive(Message, Clone)]
#[rtype(result = "Result<InternalSession, Report>")]
pub struct SaveSession(pub UserId);

message_handler_with_span! {
    impl SpanHandler<SaveSession> for SessionActor {
        type Result = ResponseActFuture<Self, <SaveSession as Message>::Result>;

        fn handle(&mut self, msg: SaveSession, _ctx: &mut Context<Self>, _span: Span) -> Self::Result {
            async {
                let user_id = msg.0.clone();
                let span = span!(Level::DEBUG, "save session", user_id = user_id.as_string().as_str());
                let _enter = span.enter();
                debug!("Saving session");
                DbExecutor::from_registry().send(SpanMessage::new(db::session::SaveSession(msg.0))).await?
            }.interop_actor_boxed(self)
        }
    }
}
