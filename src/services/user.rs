use crate::span::SpanHandler;
use crate::{
    managers::user::{InternalUser, UserId, UserManager},
    message_handler_with_span,
};
use actix::prelude::*;
use actix_interop::{with_ctx, FutureInterop};
use tracing::info;
use tracing::{debug, Span};

#[derive(Default)]
pub struct UserActor {
    manager: UserManager,
}

impl Actor for UserActor {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        info!("User actor started");
    }
}

impl SystemService for UserActor {}
impl Supervised for UserActor {}

// Find user

#[derive(Message, Clone)]
#[rtype(result = "Result<Option<InternalUser>, &'static str>")]
pub struct UserByUsername(pub String);

message_handler_with_span! {
    impl SpanHandler<UserByUsername> for UserActor {
        type Result = ResponseActFuture<Self, <UserByUsername as Message>::Result>;

        fn handle(&mut self, msg: UserByUsername, _ctx: &mut Context<Self>, _span: Span) -> Self::Result {
            debug!("Handling connect");
            async {
                let user = with_ctx(|a: &mut UserActor, _| a.manager.find_by_username(msg.0));
                Ok(user)
            }.interop_actor_boxed(self)
        }
    }
}

#[derive(Message, Clone)]
#[rtype(result = "Result<Option<InternalUser>, &'static str>")]
pub struct UserById(pub UserId);

message_handler_with_span! {
    impl SpanHandler<UserById> for UserActor {
        type Result = ResponseActFuture<Self, <UserById as Message>::Result>;

        fn handle(&mut self, msg: UserById, _ctx: &mut Context<Self>, _span: Span) -> Self::Result {
            debug!("Handling connect");
            async {
                let user = with_ctx(|a: &mut UserActor, _| a.manager.find_by_id(msg.0));
                Ok(user)
            }.interop_actor_boxed(self)
        }
    }
}
