use crate::services;
use crate::services::Service;
use actix::prelude::*;
use actix_web_actors::ws;
use serde::{Deserialize, Serialize};
use slog::{debug, error, info, o, warn};

#[derive(Deserialize)]
#[serde(tag = "type")]
enum IncomingMessage {
    Message { message: String },
}

#[derive(Serialize)]
enum IssueState {
    #[serde(rename = "notstarted")]
    NotStarted,
    #[serde(rename = "inprogress")]
    InProgress,
    #[serde(rename = "finished")]
    Finished,
}

#[derive(Serialize)]
struct Alternative {
    id: String,
    title: String,
}

#[derive(Serialize)]
struct Vote {
    id: String,
    alternative_id: String,
}

#[derive(Serialize)]
pub struct Issue {
    id: String,
    title: String,
    description: String,
    state: IssueState,
    alternatives: Vec<Alternative>,
    votes: Vec<Vote>,
    max_voters: u32,
    show_distribution: bool,
}

#[derive(Serialize)]
#[serde(tag = "type")]
enum OutgoingMessage {
    #[serde(rename = "issue")]
    Issue(Issue),
}

type WsClientId = usize;

pub struct WsClient {
    id: WsClientId,
    service: Addr<Service>,
    logger: slog::Logger,
}

impl WsClient {
    pub fn new(service: Addr<Service>, logger: slog::Logger) -> WsClient {
        let id = 0; // TODO: something smart
        return WsClient {
            id,
            service,
            logger: logger.new(o!("id" => id)),
        };
    }
    fn send_json<T: Serialize>(&self, ctx: &mut ws::WebsocketContext<Self>, value: &T) {
        match serde_json::to_string(value) {
            Ok(json) => ctx.text(json),
            Err(err) => error!(
                self.logger,
                "Failed to convert to JSON {error}",
                error = err.to_string()
            ),
        }
    }
}

impl Actor for WsClient {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        info!(self.logger, "New ws client");
        let addr = ctx.address();
        self.service.do_send(services::Connect {
            addr: addr.recipient(),
        });
    }

    fn stopped(&mut self, _ctx: &mut Self::Context) {
        info!(self.logger, "Ws client left");
    }
}

// Incoming messages from ws
impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for WsClient {
    fn handle(&mut self, _msg: Result<ws::Message, ws::ProtocolError>, _ctx: &mut Self::Context) {
        warn!(
            self.logger,
            "Handling of incoming WS messages is not implemented"
        );
    }
}

impl Handler<services::ActiveIssue> for WsClient {
    type Result = ();

    fn handle(&mut self, msg: services::ActiveIssue, ctx: &mut Self::Context) {
        debug!(self.logger, "Handling ActiveIssue event");
        let issue = msg.0;
        self.send_json(
            ctx,
            &OutgoingMessage::Issue(Issue {
                id: issue.id,
                title: issue.title,
                description: issue.description,
                state: match issue.state {
                    services::issue::InternalIssueState::NotStarted => IssueState::NotStarted,
                    services::issue::InternalIssueState::InProgress => IssueState::InProgress,
                    services::issue::InternalIssueState::Finished => IssueState::Finished,
                },
                alternatives: issue
                    .alternatives
                    .into_iter()
                    .map(|alt: services::issue::InternalAlternative| Alternative {
                        id: alt.id,
                        title: alt.title,
                    })
                    .collect(),
                votes: issue
                    .votes
                    .into_iter()
                    .map(|vote: services::issue::InternalVote| Vote {
                        id: vote.id,
                        alternative_id: vote.alternative_id,
                    })
                    .collect(),
                max_voters: issue.max_voters,
                show_distribution: issue.show_distribution,
            }),
        )
    }
}
