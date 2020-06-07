use crate::span::{ActorFutureSpanWrap, SpanMessage};
use crate::websocket::WsClient;
use actix::prelude::*;
use actix_interop::{critical_section, with_ctx, FutureInterop};
use std::fmt;
use tracing::{debug, error, info, instrument, Span};

pub mod broadcast;
pub mod client;
pub mod issue;
pub mod vote;

#[derive(Message)]
#[rtype(result = "()")]
pub struct ActiveIssue(pub issue::InternalIssue);

#[derive(Message, Clone)]
#[rtype(result = "Result<(), ()>")]
pub struct Connect {
    pub addr: Addr<WsClient>,
}

impl fmt::Debug for Connect {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Connect").finish()
    }
}

#[derive(Message, Clone)]
#[rtype(result = "()")]
pub struct Login {
    pub user_id: String,
    pub username: String,
}

#[derive(Message, Clone)]
#[rtype(result = "()")]
pub struct Disconnect {
    pub addr: Addr<WsClient>,
}

pub struct Service {
    issue_service: Addr<issue::IssueService>,
}

impl fmt::Debug for Service {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Service").finish()
    }
}

impl Service {
    pub fn new() -> Service {
        Service {
            issue_service: issue::IssueService::mocked().start(),
        }
    }
}

impl Default for Service {
    fn default() -> Self {
        unimplemented!("Service actor can't be unitialized using default because it needs a logger")
    }
}

impl Actor for Service {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        info!("Service actor started");
    }

    fn stopped(&mut self, _ctx: &mut Self::Context) {
        info!("Service actor stopped");
    }
}

#[instrument]
async fn handle_connect(msg: Connect) -> Result<(), ()> {
    info!("Test test");
    let res = with_ctx(|a: &mut Service, _| a.issue_service.send(issue::ActiveIssue)).await;
    match res {
        Ok(issue) => {
            let _res = msg.addr.send(ActiveIssue(issue)).await;
            // Send existing vote ?
            // no because we havent logged in yet
        }
        Err(err) => {
            error!(
                "Got error response. TODO check what this actually means {:#?}",
                err
            );
        }
    }
    Ok(())
}

impl Handler<Connect> for Service {
    type Result = ResponseActFuture<Self, <Connect as Message>::Result>;

    // #[instrument]
    fn handle(&mut self, msg: Connect, _ctx: &mut Context<Self>) -> Self::Result {
        debug!("Handling connect");
        handle_connect(msg).interop_actor_boxed(self)
    }
}

impl Handler<SpanMessage<Connect>> for Service {
    type Result = ResponseActFuture<Self, <Connect as Message>::Result>;
    fn handle(&mut self, msg: SpanMessage<Connect>, ctx: &mut Context<Self>) -> Self::Result {
        let SpanMessage { span, msg } = msg;
        let _enter = span.enter();
        debug!("Running wrapped span message handler");
        Box::new(ActorFutureSpanWrap::new(
            self.handle(msg, ctx),
            span.clone(),
        ))
    }
}

impl Handler<Disconnect> for Service {
    type Result = ();

    fn handle(&mut self, msg: Disconnect, ctx: &mut Context<Self>) {
        ctx.stop();
    }
}

impl Supervised for Service {}
impl ArbiterService for Service {}
