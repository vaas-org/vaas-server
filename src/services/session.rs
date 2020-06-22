use crate::span::SpanHandler;
use crate::{
    managers::session::{InternalSession, SessionId, SessionManager},
    message_handler_with_span,
};
use actix::prelude::*;
use actix_interop::{with_ctx, FutureInterop};
use tracing::info;
use tracing::{debug, Span};

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
#[rtype(result = "Result<Option<InternalSession>, &'static str>")]
pub struct SessionById(pub SessionId);

message_handler_with_span! {
    impl SpanHandler<SessionById> for SessionActor {
        type Result = ResponseActFuture<Self, <SessionById as Message>::Result>;

        fn handle(&mut self, msg: SessionById, _ctx: &mut Context<Self>, _span: Span) -> Self::Result {
            debug!("Handling session by id");
            async {
                let session = with_ctx(|a: &mut SessionActor, _| a.manager.find_by_id(msg.0));
                Ok(session)
            }.interop_actor_boxed(self)
        }
    }
}
#[derive(Message, Clone)]
#[rtype(result = "Result<(), &'static str>")]
pub struct SaveSession(pub InternalSession);

message_handler_with_span! {
    impl SpanHandler<SaveSession> for SessionActor {
        type Result = ResponseActFuture<Self, <SaveSession as Message>::Result>;

        fn handle(&mut self, msg: SaveSession, _ctx: &mut Context<Self>, _span: Span) -> Self::Result {
            debug!("Handling session by id");
            async {
                with_ctx(|a: &mut SessionActor, _| a.manager.insert(msg.0));
                Ok(())
            }.interop_actor_boxed(self)
        }
    }
}
