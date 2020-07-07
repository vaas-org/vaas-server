use actix::dev::MessageResponse;
use actix::prelude::*;
use async_trait::async_trait;
use pin_project::pin_project;
use std::pin::Pin;
use std::task::{Context, Poll};
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

pub trait SpanHandler<M>
where
    Self: Actor,
    M: Message,
{
    /// The type of value that this handler will return.
    type Result: MessageResponse<Self, M>;

    /// This method is called for every message received by this actor.
    fn handle(&mut self, msg: M, ctx: &mut Self::Context, span: Span) -> Self::Result;
}

#[async_trait]
pub trait AsyncSpanHandler<M>
where
    Self: Actor,
    M: Message,
{
    async fn handle(msg: M) -> <M as Message>::Result;
}

/// ActorFuture wrapper that enters span before polling future
#[pin_project]
#[derive(Debug)]
pub struct ActorFutureSpanWrap<F> {
    #[pin]
    inner: F,
    span: Span,
}

impl<F: ActorFuture> ActorFutureSpanWrap<F> {
    pub fn new(inner: F, span: Span) -> Self {
        Self { inner, span }
    }
}

impl<F: ActorFuture> ActorFuture for ActorFutureSpanWrap<F> {
    type Actor = F::Actor;
    type Output = F::Output;

    fn poll(
        self: Pin<&mut Self>,
        actor: &mut Self::Actor,
        ctx: &mut <Self::Actor as Actor>::Context,
        task: &mut Context,
    ) -> Poll<Self::Output> {
        let this = self.project();
        let _enter = this.span.enter();
        this.inner.poll(actor, ctx, task)
    }
}

#[macro_export]
macro_rules! span_message_impl {
    ($message_type:ident, $actor:ident) => {
        impl Handler<crate::span::SpanMessage<$message_type>> for $actor {
            type Result = ResponseActFuture<Self, <$message_type as Message>::Result>;
            fn handle(
                &mut self,
                msg: crate::span::SpanMessage<$message_type>,
                ctx: &mut Context<Self>,
            ) -> Self::Result {
                let crate::span::SpanMessage { span, msg } = msg;
                let _enter = span.enter();
                tracing::debug!("Running wrapped span message handler");
                Box::new(crate::span::ActorFutureSpanWrap::new(
                    <Self as SpanHandler<$message_type>>::handle(self, msg, ctx, span.clone()),
                    span.clone(),
                ))
            }
        }
    };
}

#[macro_export]
macro_rules ! message_handler_with_span {
    (impl SpanHandler<$M:ident> for $A:ident $t:tt) => {
        crate::span_message_impl!($M, $A);
        impl SpanHandler<$M> for $A
            $t

    }
}

#[macro_export]
macro_rules! span_message_async_impl {
    ($message_type:ident, $actor:ident) => {
        use actix_interop::FutureInterop;
        use tracing_futures::Instrument;
        impl Handler<crate::span::SpanMessage<$message_type>> for $actor {
            type Result = ResponseActFuture<Self, <$message_type as Message>::Result>;
            fn handle(
                &mut self,
                msg: crate::span::SpanMessage<$message_type>,
                _ctx: &mut Context<Self>,
            ) -> Self::Result {
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
    (impl AsyncSpanHandler<$M:ident> for $A:ident $t:tt) => {
        crate::span_message_async_impl!($M, $A);
        #[async_trait::async_trait]
        impl AsyncSpanHandler<$M> for $A
            $t

    }
}
