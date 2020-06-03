use crate::services;
use crate::services::broadcast::BroadcastActor;
use crate::services::client::ClientActor;
use crate::services::client::{IncomingNewClient, UserId};
use crate::services::vote::{AlternativeId, BroadcastVote, IncomingVoteMessage, VoteActor};
use crate::services::{Login, Service};
use actix::prelude::*;
use actix_web_actors::ws;
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info, warn};

#[derive(Serialize, Deserialize)]
pub struct IncomingLogin {
    user_id: String,
    username: String,
}
#[derive(Serialize, Deserialize)]
pub struct IncomingVote {
    pub alternative_id: String,
    pub user_id: String, // TODO: should be removed
}
#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum IncomingMessage {
    #[serde(rename = "vote")]
    Vote(IncomingVote),
    #[serde(rename = "login")]
    Login(IncomingLogin),
}

#[derive(Serialize, Deserialize)]
enum IssueState {
    #[serde(rename = "notstarted")]
    NotStarted,
    #[serde(rename = "inprogress")]
    InProgress,
    #[serde(rename = "finished")]
    Finished,
}

#[derive(Serialize, Deserialize)]
struct Alternative {
    id: String,
    pub title: String,
}

#[derive(Serialize, Deserialize)]
pub struct OutgoingVote {
    id: String,
    pub alternative_id: String,
    pub user_id: String,
}

#[derive(Serialize, Deserialize)]
pub struct OutgoingClient {
    id: String,
    pub username: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct Issue {
    id: String,
    pub title: String,
    description: String,
    state: IssueState,
    alternatives: Vec<Alternative>,
    votes: Vec<OutgoingVote>,
    max_voters: u32,
    show_distribution: bool,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum OutgoingMessage {
    #[serde(rename = "issue")]
    Issue(Issue),
    #[serde(rename = "vote")]
    Vote(OutgoingVote),
    #[serde(rename = "client")]
    Client(OutgoingClient),
}

pub struct WsClient {}

impl WsClient {
    pub fn new() -> Self {
        Self {}
    }
    fn send_json<T: Serialize>(&self, ctx: &mut ws::WebsocketContext<Self>, value: &T) {
        match serde_json::to_string(value) {
            Ok(json) => ctx.text(json),
            Err(err) => error!("Failed to convert to JSON {error}", error = err.to_string()),
        }
    }
}

impl Actor for WsClient {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        info!("New ws client");
        let addr = ctx.address();
        let connect = services::Connect { addr };
        let service = Service::from_registry();
        service.do_send(connect.clone());
        BroadcastActor::from_registry().do_send(connect.clone());
        ClientActor::from_registry().do_send(connect);
    }

    fn stopped(&mut self, ctx: &mut Self::Context) {
        info!("Ws client left");
        let addr = ctx.address();
        BroadcastActor::from_registry().do_send(services::Disonnect { addr });
    }
}

// Incoming messages from ws
impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for WsClient {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(message) => match message {
                ws::Message::Text(text) => {
                    let m = serde_json::from_str(&text);
                    if let Err(serde_error) = m {
                        println!("JSON error {:#?}", serde_error);
                        return;
                    }
                    let m = m.unwrap();
                    match m {
                        IncomingMessage::Vote(vote) => {
                            debug!("Incoming vote");
                            let vote_actor = VoteActor::from_registry();
                            let user_id = vote.user_id; // TODO: Should not be sent by client
                            let alternative_id = vote.alternative_id;
                            vote_actor.do_send(IncomingVoteMessage(
                                UserId(user_id),
                                AlternativeId(alternative_id),
                            ));
                        }
                        IncomingMessage::Login(login) => {
                            debug!("Incoming login {} {}", login.user_id, login.username);
                            let user_actor = ClientActor::from_registry();
                            user_actor.do_send(Login {
                                user_id: login.user_id,
                                username: login.username,
                            });
                        }
                    }
                }
                ws::Message::Close(reason) => {
                    debug!("Got close message from WS. Reason: {:#?}", reason);
                    ctx.close(reason)
                }
                message => {
                    warn!("Client sent something else than text: {:#?}", message);
                }
            },
            Err(err) => {
                error!("ProtocolError in StreamHandler {:#?}", err);
            }
        }
    }
}

impl Handler<services::ActiveIssue> for WsClient {
    type Result = ();

    fn handle(&mut self, msg: services::ActiveIssue, ctx: &mut Self::Context) {
        debug!("Handling ActiveIssue event");
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
                    .map(|vote: services::vote::InternalVote| OutgoingVote {
                        id: vote.id.0,
                        alternative_id: vote.alternative_id.0,
                        user_id: vote.user_id.0,
                    })
                    .collect(),
                max_voters: issue.max_voters,
                show_distribution: issue.show_distribution,
            }),
        )
    }
}

impl Handler<BroadcastVote> for WsClient {
    type Result = ();

    fn handle(&mut self, msg: BroadcastVote, ctx: &mut Self::Context) {
        let vote = msg.0;
        self.send_json(
            ctx,
            &OutgoingMessage::Vote(OutgoingVote {
                id: vote.id.0,
                alternative_id: vote.alternative_id.0,
                user_id: vote.user_id.0,
            }),
        )
    }
}

impl Handler<IncomingNewClient> for WsClient {
    type Result = ();

    fn handle(&mut self, msg: IncomingNewClient, ctx: &mut Self::Context) {
        let client = msg;

        match client.0.username.clone() {
            Some(u) => {
                debug!(
                    "Sending client details back to client {} ({})",
                    client.0.id, u
                );
            }
            None => {
                debug!("Sending client details back to client {}", client.0.id);
            }
        };

        self.send_json(
            ctx,
            &OutgoingMessage::Client(OutgoingClient {
                id: client.0.id,
                username: client.0.username,
            }),
        )
    }
}
