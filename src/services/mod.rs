use crate::message_handler_with_span;
use crate::span::{SpanHandler, SpanMessage};
use crate::websocket::WsClient;
use actix::prelude::*;
use actix_interop::FutureInterop;
use color_eyre::eyre::{Report, WrapErr};
use issue::IssueService;
use std::fmt;
use tracing::{debug, error, info, instrument, Span};

pub mod broadcast;
pub mod client;
pub mod issue;
pub mod session;
pub mod vote;

#[derive(Message)]
#[rtype(result = "()")]
pub struct ActiveIssue(pub issue::InternalIssue);

#[derive(Message, Clone)]
#[rtype(result = "Result<(), Report>")]
pub struct Connect {
    pub addr: Addr<WsClient>,
}

impl fmt::Debug for Connect {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Connect").finish()
    }
}

#[derive(Message, Clone)]
#[rtype(result = "Result<Option<crate::db::user::InternalUser>, Report>")]
pub struct Login {
    pub username: String,
}

#[derive(Message, Clone)]
#[rtype(result = "()")]
pub struct Disconnect {
    pub addr: Addr<WsClient>,
}

pub struct Service {}

impl fmt::Debug for Service {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Service").finish()
    }
}

impl Service {
    pub fn new() -> Service {
        Service {}
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
async fn handle_connect(msg: Connect, span: Span) -> Result<(), Report> {
    info!("Test test");
    let res = IssueService::from_registry()
        .send(SpanMessage::new(issue::ActiveIssue, span))
        .await??;
    match res {
        Some(issue) => {
            msg.addr
                .send(ActiveIssue(issue))
                .await
                .wrap_err("Failed to send active issue")?;
            // Send existing vote ?
            // no because we havent logged in yet
        }
        None => {
            error!("No active issue found! Missing sample data?");
        }
    }
    Ok(())
}

message_handler_with_span! {
    impl SpanHandler<Connect> for Service {
        type Result = ResponseActFuture<Self, <Connect as Message>::Result>;

        fn handle(&mut self, msg: Connect, _ctx: &mut Context<Self>, span: Span) -> Self::Result {
            debug!("Handling connect");
            handle_connect(msg, span).interop_actor_boxed(self)
        }
    }
}

impl Handler<Disconnect> for Service {
    type Result = ();

    fn handle(&mut self, _msg: Disconnect, ctx: &mut Context<Self>) {
        ctx.stop();
    }
}

impl Supervised for Service {}
impl ArbiterService for Service {}
