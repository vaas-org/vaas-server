use actix::prelude::*;
use async_trait::async_trait;
use tracing::Span;

/// Message with span used for trace logging
pub struct SpanMessage<I> {
    pub msg: I,
    pub span: Span,
}

impl<M> SpanMessage<M> {
    pub fn new(msg: M) -> Self {
        Self {
            msg,
            span: Span::current(),
        }
    }
}

impl<M, R: 'static> Message for SpanMessage<M>
where
    M: Message<Result = R>,
{
    type Result = R;
}

#[async_trait]
pub trait AsyncSpanHandler<M>
where
    Self: Actor,
    M: Message,
{
    async fn handle(msg: M) -> <M as Message>::Result;
}

#[macro_export]
macro_rules! span_message_async_impl {
    ($message_type:ident, $actor:ident) => {
        impl actix::Handler<crate::span::SpanMessage<$message_type>> for $actor {
            type Result = actix::ResponseActFuture<Self, <$message_type as actix::Message>::Result>;
            fn handle(
                &mut self,
                msg: crate::span::SpanMessage<$message_type>,
                _ctx: &mut actix::Context<Self>,
            ) -> Self::Result {
                use actix_interop::FutureInterop;
                use tracing_futures::Instrument;
                let crate::span::SpanMessage { span, msg } = msg;
                let _enter = span.enter();
                <Self as AsyncSpanHandler<$message_type>>::handle(msg)
                    .in_current_span()
                    .interop_actor_boxed(self)
            }
        }
    };
}

#[macro_export]
macro_rules ! async_message_handler_with_span {
    ({impl AsyncSpanHandler<$M:ident> for $A:ident $t:tt}) => {
        crate::span_message_async_impl!($M, $A);
        #[async_trait::async_trait]
        impl AsyncSpanHandler<$M> for $A
            $t

    }
}
