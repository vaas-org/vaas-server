use crate::websocket::WsClient;
use actix::prelude::*;
use actix_interop::{critical_section, with_ctx, FutureInterop};
use tracing::{debug, error, info};

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

impl Handler<Connect> for Service {
    type Result = ResponseActFuture<Self, <Connect as Message>::Result>;

    fn handle(&mut self, msg: Connect, _ctx: &mut Context<Self>) -> Self::Result {
        async move {
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
        .interop_actor_boxed(self)
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
