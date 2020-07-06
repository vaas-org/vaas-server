use crate::services;
use crate::services::broadcast::BroadcastActor;
use crate::services::client::ClientActor;
use crate::services::vote::{BroadcastVote, IncomingVoteMessage, VoteActor};
use crate::services::{Login, Service};
use crate::{
    db::DbExecutor,
    db::{
        self,
        user::{UserById, UserId},
    },
    managers::session::{InternalSession, SessionId},
    span::SpanMessage,
};
use actix::prelude::*;
use actix_interop::{with_ctx, FutureInterop};
use actix_web_actors::ws;
use db::{
    alternative::AlternativeId,
    issue::IssueId,
    vote::{InternalVote, VoteId},
};
use serde::{Deserialize, Serialize};
use services::session::{SaveSession, SessionActor, SessionById};
use tracing::{debug, error, info, span, warn, Level};

#[derive(Serialize, Deserialize)]
pub struct IncomingLogin {
    pub username: String,
}
#[derive(Serialize, Deserialize)]
pub struct IncomingVote {
    pub alternative_id: AlternativeId,
    pub user_id: Option<UserId>, // Used for fake voting
}
#[derive(Serialize, Deserialize)]
pub struct IncomingReconnect {
    pub session_id: SessionId,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum IncomingMessage {
    #[serde(rename = "vote")]
    Vote(IncomingVote),
    #[serde(rename = "login")]
    Login(IncomingLogin),
    #[serde(rename = "reconnect")]
    Reconnect(IncomingReconnect),
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
    id: AlternativeId,
    pub title: String,
}

#[derive(Serialize, Deserialize)]
pub struct OutgoingVote {
    id: VoteId,
    pub alternative_id: AlternativeId,
    pub user_id: UserId,
}

#[derive(Serialize, Deserialize)]
pub struct OutgoingClient {
    pub id: SessionId,
    pub username: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct Issue {
    id: IssueId,
    pub title: String,
    description: String,
    state: IssueState,
    alternatives: Vec<Alternative>,
    votes: Vec<OutgoingVote>,
    max_voters: i32,
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

pub struct WsClient {
    session_id: Option<SessionId>,
    user_id: Option<UserId>,
}

impl WsClient {
    pub fn new() -> Self {
        Self {
            session_id: None,
            user_id: None,
        }
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
        let span = span!(Level::INFO, "ws connect");
        let _enter = span.enter();
        info!("New ws client");
        let addr = ctx.address();
        let connect = services::Connect { addr };
        let service = Service::from_registry();
        service.do_send(SpanMessage::new(connect.clone(), span.clone()));
        BroadcastActor::from_registry().do_send(connect);
    }

    fn stopped(&mut self, ctx: &mut Self::Context) {
        info!("Ws client left");
        let addr = ctx.address();
        let disconnect = services::Disconnect { addr };
        BroadcastActor::from_registry().do_send(disconnect);
        // Service::from_registry().do_send(disconnect);
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
                            let user_id = if let Some(user_id) = self.user_id.clone() {
                                user_id
                            } else if let Some(user_id) = vote.user_id {
                                // Fake user vote from frontend
                                user_id
                            } else {
                                error!("Tried to vote before logging in");
                                return;
                            };
                            let alternative_id = vote.alternative_id;
                            vote_actor.do_send(IncomingVoteMessage(user_id, alternative_id));
                        }
                        IncomingMessage::Login(login) => {
                            let span = span!(Level::INFO, "login");
                            let outer_span = span.clone();
                            let _enter = outer_span.enter();
                            debug!("Incoming login {}", login.username);
                            let user_actor = ClientActor::from_registry();
                            user_actor
                                .send(SpanMessage::new(
                                    Login {
                                        username: login.username,
                                    },
                                    span.clone(),
                                ))
                                .into_actor(self)
                                .then(move |res, act: &mut WsClient, ctx| {
                                    let user = res.unwrap().unwrap();
                                    if let Some(user) = user {
                                        let session_id = SessionId::new();
                                        let session_actor = SessionActor::from_registry();
                                        session_actor.do_send(SpanMessage::new(
                                            SaveSession(InternalSession {
                                                id: session_id.clone(),
                                                user_id: user.uuid.clone(),
                                            }),
                                            span,
                                        ));
                                        act.session_id = Some(session_id.clone());
                                        act.user_id = Some(user.uuid);
                                        act.send_json(
                                            ctx,
                                            &OutgoingMessage::Client(OutgoingClient {
                                                id: session_id,
                                                username: Some(user.username),
                                            }),
                                        );
                                    }
                                    fut::ready(())
                                })
                                .spawn(ctx);
                        }
                        IncomingMessage::Reconnect(IncomingReconnect { session_id }) => {
                            let span =
                                span!(Level::INFO, "reconnect", session_id = session_id.0.as_str());
                            let outer_span = span.clone();
                            let _enter = outer_span.enter();
                            debug!("Incoming reconnect");
                            ctx.spawn(
                                async move {
                                    let session_actor = SessionActor::from_registry();
                                    let res = session_actor
                                        .send(SpanMessage::new(
                                            SessionById(session_id.clone()),
                                            span.clone(),
                                        ))
                                        .await;
                                    let session: Option<InternalSession> = res.unwrap().unwrap();
                                    if let Some(session) = session {
                                        info!("Found session");
                                        with_ctx(|act: &mut WsClient, _| {
                                            act.session_id = Some(session.id.clone());
                                            act.user_id = Some(session.user_id.clone());
                                        });
                                        let db_executor = DbExecutor::from_registry();
                                        let user = db_executor
                                            .send(SpanMessage::new(
                                                UserById(session.user_id),
                                                span.clone(),
                                            ))
                                            .await;
                                        if let Some(user) = user.unwrap().unwrap() {
                                            info!("Found user, sending client info");
                                            with_ctx(|act: &mut WsClient, ctx| {
                                                act.send_json(
                                                    ctx,
                                                    &OutgoingMessage::Client(OutgoingClient {
                                                        id: session_id,
                                                        username: Some(user.username),
                                                    }),
                                                )
                                            });
                                        } else {
                                            error!("Unable to find user connected to session");
                                        }
                                    } else {
                                        warn!("Unable to find session");
                                    }
                                }
                                .interop_actor_boxed(self),
                            );
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
                    db::issue::InternalIssueState::NotStarted => IssueState::NotStarted,
                    db::issue::InternalIssueState::InProgress => IssueState::InProgress,
                    db::issue::InternalIssueState::Finished => IssueState::Finished,
                },
                alternatives: issue
                    .alternatives
                    .into_iter()
                    .map(|alt: db::alternative::InternalAlternative| Alternative {
                        id: alt.id,
                        title: alt.title,
                    })
                    .collect(),
                votes: issue
                    .votes
                    .into_iter()
                    .map(|vote: InternalVote| OutgoingVote {
                        id: vote.id,
                        alternative_id: vote.alternative_id,
                        user_id: vote.user_id,
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
                id: vote.id,
                alternative_id: vote.alternative_id,
                user_id: vote.user_id,
            }),
        )
    }
}
