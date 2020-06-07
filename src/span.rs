use actix::prelude::*;
use pin_project::pin_project;
use std::pin::Pin;
use std::task::{Context, Poll};
use tracing::Span;

/// Message with span used for trace logging
pub struct SpanMessage<I> {
    pub msg: I,
    pub span: Span,
}

impl<I> SpanMessage<I> {
    pub fn new(msg: I, span: Span) -> Self {
        Self { msg, span }
    }
}

impl<M, R: 'static> Message for SpanMessage<M>
where
    M: Message<Result = R>,
{
    type Result = R;
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
