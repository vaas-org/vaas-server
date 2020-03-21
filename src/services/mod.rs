use actix::prelude::*;
use slog::{debug, error};

pub mod issue;

#[derive(Message)]
#[rtype(result = "()")]
pub struct ActiveIssue(pub issue::InternalIssue);

#[derive(Message)]
#[rtype(result = "()")]
pub struct Connect {
    pub addr: Recipient<ActiveIssue>,
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
                        let send_result = msg.addr.do_send(ActiveIssue(issue));
                        if let Err(err) = send_result {
                            error!(act.logger, "Done goofed while sending issue: {:#?}", err);
                        }
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
            .spawn(ctx)
    }
}
