use crate::service;
use crate::service::Service;
use actix::prelude::*;
use actix_web_actors::ws;
use serde::{Deserialize, Serialize};

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
#[serde(untagged)]
enum OutgoingMessage {
    Issue(Issue),
}

type WsClientId = usize;

pub struct WsClient {
    pub id: WsClientId,
    pub service: Addr<Service>,
}

impl WsClient {
    fn send_json<T: Serialize>(&self, ctx: &mut ws::WebsocketContext<Self>, value: &T) {
        match serde_json::to_string(value) {
            Ok(json) => ctx.text(json),
            Err(error) => println!("Failed to convert to JSON {:#?}", error),
        }
    }
}

impl Actor for WsClient {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        println!("New ws client {:?}", self.id);
        let addr = ctx.address();
        self.service.do_send(service::Connect {
            addr: addr.recipient(),
        });
    }

    fn stopped(&mut self, _ctx: &mut Self::Context) {
        println!("Ws client left {:?}", self.id);
    }
}

// Incoming messages from ws
impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for WsClient {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {}
}

impl Handler<service::ActiveIssue> for WsClient {
    type Result = ();

    fn handle(&mut self, msg: service::ActiveIssue, ctx: &mut Self::Context) {
        println!("Handling ActiveIssue event");
        let issue = msg.0;
        self.send_json(
            ctx,
            &OutgoingMessage::Issue(Issue {
                id: issue.id,
                title: issue.title,
                description: issue.description,
                state: match issue.state {
                    service::InternalIssueState::NotStarted => IssueState::NotStarted,
                    service::InternalIssueState::InProgress => IssueState::InProgress,
                    service::InternalIssueState::Finished => IssueState::Finished,
                },
                alternatives: issue
                    .alternatives
                    .into_iter()
                    .map(|alt: service::InternalAlternative| Alternative {
                        id: alt.id,
                        title: alt.title,
                    })
                    .collect(),
                votes: issue
                    .votes
                    .into_iter()
                    .map(|vote: service::InternalVote| Vote {
                        id: vote.id,
                        alternative_id: vote.alternative_id,
                    })
                    .collect(),
                max_voters: issue.max_voters,
                show_distribution: issue.show_distribution
            }),
        )
    }
}
