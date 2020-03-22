use crate::services;
use crate::services::broadcast::BroadcastActor;
use crate::services::vote::{AlternativeId, BroadcastVote, IncomingVoteMessage, VoteActor};
use crate::services::user::{IncomingLoginMessage, UserActor, UserId};
use crate::services::Service;
use actix::prelude::*;
use actix_web_actors::ws;
use serde::{Deserialize, Serialize};
use slog::{debug, error, info, o, warn};

#[derive(Deserialize)]
struct IncomingLogin {
    username: String,
}
#[derive(Deserialize)]
struct IncomingVote {
    alternative_id: String,
    user_id: String, // TODO: should be removed
}
#[derive(Deserialize)]
#[serde(tag = "type")]
enum IncomingMessage {
    #[serde(rename = "vote")]
    Vote(IncomingVote),
    #[serde(rename = "login")]
    Login(IncomingLogin)
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
struct OutgoingVote {
    id: String,
    alternative_id: String,
    user_id: String,
}

#[derive(Serialize)]
struct Issue {
    id: String,
    title: String,
    description: String,
    state: IssueState,
    alternatives: Vec<Alternative>,
    votes: Vec<OutgoingVote>,
    max_voters: u32,
    show_distribution: bool,
}

#[derive(Serialize)]
#[serde(tag = "type")]
enum OutgoingMessage {
    #[serde(rename = "issue")]
    Issue(Issue),
    #[serde(rename = "vote")]
    Vote(OutgoingVote),
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
        let connect = services::Connect { addr };
        self.service.do_send(connect.clone());
        BroadcastActor::from_registry().do_send(connect);
    }

    fn stopped(&mut self, ctx: &mut Self::Context) {
        info!(self.logger, "Ws client left");
        let addr = ctx.address();
        BroadcastActor::from_registry().do_send(services::Disonnect { addr });
    }
}

// Incoming messages from ws
impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for WsClient {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, _ctx: &mut Self::Context) {
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
                            debug!(self.logger, "Incoming vote");
                            let vote_actor = VoteActor::from_registry();
                            let user_id = vote.user_id; // TODO: Should not be sent by client
                            let alternative_id = vote.alternative_id;
                            vote_actor.do_send(IncomingVoteMessage(
                                UserId(user_id),
                                AlternativeId(alternative_id),
                            ));
                        }
                        IncomingMessage::Login(login) => {
                            debug!(self.logger, "Incoming login");
                            let user_actor = UserActor::from_registry();
                            let username = login.username;
                            user_actor.do_send(IncomingLoginMessage(
                                services::user::UserId("aaa".to_string()),
                                username.clone()
                            ));
                        }
                    }
                }
                _ => {
                    warn!(self.logger, "Client sent something else than text");
                }
            },
            Err(err) => {
                error!(self.logger, "ProtocolError in StreamHandler {:#?}", err);
            }
        }
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
