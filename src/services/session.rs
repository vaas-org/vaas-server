use crate::span::SpanHandler;
use crate::{
    managers::session::{InternalSession, SessionId, SessionManager},
    message_handler_with_span,
};
use actix::prelude::*;
use actix_interop::{with_ctx, FutureInterop};
use color_eyre::eyre::Report;
use tracing::info;
use tracing::{debug, span, Level, Span};

#[derive(Default)]
pub struct SessionActor {
    manager: SessionManager,
}

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

        fn handle(&mut self, msg: SessionById, _ctx: &mut Context<Self>, _span: Span) -> Self::Result {
            async {
                debug!("Handling session by id");
                let session = with_ctx(|a: &mut SessionActor, _| a.manager.find_by_id(msg.0));
                Ok(session)
            }.interop_actor_boxed(self)
        }
    }
}
#[derive(Message, Clone)]
#[rtype(result = "Result<(), Report>")]
pub struct SaveSession(pub InternalSession);

message_handler_with_span! {
    impl SpanHandler<SaveSession> for SessionActor {
        type Result = ResponseActFuture<Self, <SaveSession as Message>::Result>;

        fn handle(&mut self, msg: SaveSession, _ctx: &mut Context<Self>, _span: Span) -> Self::Result {
            async {
                let user_id = msg.0.user_id.clone();
                let span = span!(Level::DEBUG, "save session", user_id = user_id.as_string().as_str());
                let _enter = span.enter();
                debug!("Saving session");
                with_ctx(|a: &mut SessionActor, _| a.manager.insert(msg.0));
                Ok(())
            }.interop_actor_boxed(self)
        }
    }
}
