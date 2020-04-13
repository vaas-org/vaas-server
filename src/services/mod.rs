use crate::websocket::WsClient;
use actix::prelude::*;
use slog::{debug, error, info};

pub mod broadcast;
pub mod client;
pub mod issue;
pub mod vote;

#[derive(Message)]
#[rtype(result = "()")]
pub struct ActiveIssue(pub issue::InternalIssue);

#[derive(Message, Clone)]
#[rtype(result = "()")]
pub struct Connect {
    pub addr: Addr<WsClient>,
}

#[derive(Message, Clone)]
#[rtype(result = "()")]
pub struct Login {
    pub user_id: String,
    pub username: String,
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct Disonnect {
    // 😂
    pub addr: Addr<WsClient>,
}

pub struct Service {
    logger: slog::Logger,
    issue_service: Addr<issue::IssueService>,
}

impl Service {
    pub fn new(logger: slog::Logger) -> Service {
        Service {
            logger,
            issue_service: issue::IssueService::mocked().start(),
        }
    }
}

impl Actor for Service {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        info!(self.logger, "Service actor started");
    }
}

impl Handler<Connect> for Service {
    type Result = ();

    fn handle(&mut self, msg: Connect, ctx: &mut Context<Self>) {
        debug!(self.logger, "Handling connect");

        self.issue_service
            .send(issue::ActiveIssue)
            .into_actor(self)
            .then(move |res, act, _ctx| {
                match res {
                    Ok(issue) => {
                        msg.addr.do_send(ActiveIssue(issue));
                        // Send existing vote ?
                        // no because we havent logged in yet
                    }
                    Err(err) => {
                        error!(
                            act.logger,
                            "Got error response. TODO check what this actually means {:#?}", err
                        );
                    }
                }
                fut::ready(())
            })
            .spawn(ctx);
    }
}
