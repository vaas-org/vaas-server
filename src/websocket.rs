use crate::services;
use crate::services::broadcast::BroadcastActor;
use crate::services::client::ClientActor;
use crate::services::issue::{IssueService, ListAllIssues, NewIssue};
use crate::services::vote::{BroadcastVote, IncomingVoteMessage, VoteActor};
use crate::services::{Login, Service};
use crate::{db, db::DbExecutor, span::SpanMessage};
use actix::prelude::*;
use actix_interop::{with_ctx, FutureInterop};
use actix_web_actors::ws;
use color_eyre::eyre::{eyre, Report, WrapErr};
use db::{
    alternative::AlternativeId,
    issue::IssueId,
    session::{InternalSession, SessionId},
    user::NewInternalUser,
    user::NewUser,
    user::{UserById, UserId},
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
    pub issue_id: IssueId,
    pub user_id: Option<UserId>, // Used for fake voting
}
#[derive(Serialize, Deserialize)]
pub struct IncomingCreateIssue {
    pub issue: Issue,
}
#[derive(Serialize, Deserialize)]
pub struct IncomingReconnect {
    pub session_id: SessionId,
}

#[derive(Serialize, Deserialize)]
pub struct IncomingRegistration {
    pub username: String,
}

#[derive(Serialize, Deserialize)]
pub struct IncomingListAllIssues;

#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum IncomingMessage {
    #[serde(rename = "vote")]
    Vote(IncomingVote),
    #[serde(rename = "login")]
    Login(IncomingLogin),
    #[serde(rename = "reconnect")]
    Reconnect(IncomingReconnect),
    #[serde(rename = "issue_create")]
    CreateIssue(IncomingCreateIssue),
    #[serde(rename = "registration")]
    Registration(IncomingRegistration),
    #[serde(rename = "list_all_issues")]
    ListAllIssues,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum IssueState {
    #[serde(rename = "notstarted")]
    NotStarted,
    #[serde(rename = "inprogress")]
    InProgress,
    #[serde(rename = "votingfinished")]
    VotingFinished,
    #[serde(rename = "finished")]
    Finished,
}

impl IssueState {
    pub fn to_internal(self) -> db::issue::InternalIssueState {
        return match self {
            IssueState::NotStarted => db::issue::InternalIssueState::NotStarted,
            IssueState::InProgress => db::issue::InternalIssueState::InProgress,
            IssueState::VotingFinished => db::issue::InternalIssueState::VotingFinished,
            IssueState::Finished => db::issue::InternalIssueState::Finished,
        };
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Alternative {
    pub id: Option<AlternativeId>,
    pub title: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
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

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Issue {
    pub id: Option<IssueId>,
    pub title: String,
    pub description: String,
    pub state: Option<IssueState>,
    pub alternatives: Vec<Alternative>,
    pub votes: Option<Vec<OutgoingVote>>,
    pub max_voters: Option<i32>,
    pub show_distribution: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Issues {
    pub issues: Vec<Issue>,
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
    #[serde(rename = "all_issues")]
    AllIssues(Issues),
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
    fn send_json<T: Serialize>(
        &self,
        ctx: &mut ws::WebsocketContext<Self>,
        value: &T,
    ) -> Result<(), Report> {
        let json = serde_json::to_string(value).wrap_err("Failed to convert to JSON")?;
        ctx.text(json);
        Ok(())
    }
}

impl Default for WsClient {
    fn default() -> Self {
        Self::new()
    }
}

fn report_error(report: Report) {
    error!("Error report: {:?}", report);
}

async fn handle_vote(vote: IncomingVote) -> Result<(), Report> {
    let span = span!(Level::DEBUG, "vote", alternative_id = ?vote.alternative_id);
    let _enter = span.enter();
    debug!("Incoming vote");
    let vote_actor = VoteActor::from_registry();
    let user_id = if let Some(user_id) = with_ctx(|act: &mut WsClient, _| act.user_id.clone()) {
        user_id
    } else if let Some(user_id) = vote.user_id {
        // Fake user vote from frontend
        user_id
    } else {
        return Err(eyre!("Tried to vote before logging in"));
    };
    let alternative_id = vote.alternative_id;
    vote_actor
        .send(SpanMessage::new(IncomingVoteMessage(
            user_id,
            vote.issue_id,
            alternative_id,
        )))
        .await
        .wrap_err("Error handling incoming vote")?
}

async fn handle_login(login: IncomingLogin) -> Result<(), Report> {
    let span = span!(Level::INFO, "login");
    let outer_span = span.clone();
    let _enter = outer_span.enter();
    debug!("Incoming login {}", login.username);
    let user_actor = ClientActor::from_registry();
    let res = user_actor
        .send(SpanMessage::new(Login {
            username: login.username,
        }))
        .await;
    let user = res??;
    if let Some(user) = user {
        let session_actor = SessionActor::from_registry();
        let session = session_actor
            .send(SpanMessage::new(SaveSession(user.id.clone())))
            .await??;
        with_ctx(|act: &mut WsClient, ctx| {
            act.session_id = Some(session.id.clone());
            act.user_id = Some(user.id);
            act.send_json(
                ctx,
                &OutgoingMessage::Client(OutgoingClient {
                    id: session.id,
                    username: Some(user.username),
                }),
            )
        })
        .wrap_err("Failed to send client message on login")?;
    }
    Ok(())
}

async fn handle_reconnect(
    IncomingReconnect { session_id }: IncomingReconnect,
) -> Result<(), Report> {
    let span = span!(
        Level::INFO,
        "reconnect",
        session_id = session_id.as_string().as_str()
    );
    let outer_span = span.clone();
    let _enter = outer_span.enter();
    debug!("Incoming reconnect");
    let session_actor = SessionActor::from_registry();
    let res = session_actor
        .send(SpanMessage::new(SessionById(session_id.clone())))
        .await;
    let session: Option<InternalSession> = res??;
    if let Some(session) = session {
        info!("Found session");
        with_ctx(|act: &mut WsClient, _| {
            act.session_id = Some(session.id.clone());
            act.user_id = Some(session.user_id.clone());
        });
        let db_executor = DbExecutor::from_registry();
        let user = db_executor
            .send(SpanMessage::new(UserById(session.user_id)))
            .await;
        if let Some(user) = user?? {
            info!("Found user, sending client info");
            with_ctx(|act: &mut WsClient, ctx| {
                act.send_json(
                    ctx,
                    &OutgoingMessage::Client(OutgoingClient {
                        id: session_id,
                        username: Some(user.username),
                    }),
                )
            })
            .wrap_err("Failed to send client message on reconnect")?;
        } else {
            error!("Unable to find user connected to session");
        }
    } else {
        warn!("Unable to find session");
    }
    Ok(())
}

async fn handle_create_issue(
    IncomingCreateIssue { issue }: IncomingCreateIssue,
) -> Result<(), Report> {
    let span = span!(Level::DEBUG, "issue_create", issue = issue.title.as_str());
    let _enter = span.enter();
    debug!("Incoming CreateIssue");
    let issue_actor = IssueService::from_registry();
    let _ = if let Some(user_id) = with_ctx(|act: &mut WsClient, _| act.user_id.clone()) {
        user_id
    } else {
        return Err(eyre!(
            "Not sure who the user who tried to create an issue is"
        ));
    };
    // @ToDo: Check if user is admin
    let resp = issue_actor
        .send(SpanMessage::new(NewIssue(issue)))
        .await
        .wrap_err("Error handling incoming new issue")?;
    match resp {
        Ok(_) => Ok(()), // respond back with issue maybe?
        _ => Ok(()),     // how to handle error hmm
    }
}

#[derive(Serialize, Deserialize)]
struct User {
    pub id: UserId,
    pub username: String,
}

async fn handle_registration(registration: IncomingRegistration) -> Result<(), Report> {
    let span = span!(
        Level::DEBUG,
        "registration",
        username = registration.username.as_str()
    );
    let _enter = span.enter();
    debug!("Incoming UserRegistration");
    let db_executor = DbExecutor::from_registry();
    let user = db_executor
        .send(SpanMessage::new(NewUser(NewInternalUser {
            username: registration.username,
        })))
        .await??;
    info!("Successfully registered user {:?}", user);
    let session_actor = SessionActor::from_registry();
    let session = session_actor
        .send(SpanMessage::new(SaveSession(user.id.clone())))
        .await??;
    debug!("Created session");
    with_ctx(|act: &mut WsClient, ctx| {
        act.user_id = Some(user.id);
        act.session_id = Some(session.id.clone());
        act.send_json(
            ctx,
            &OutgoingMessage::Client(OutgoingClient {
                id: session.id,
                username: Some(user.username),
            }),
        )
    })
    .wrap_err("Failed to send client message on login")?;
    Ok(())
}

async fn handle_list_all_issues() -> Result<(), Report> {
    let span = span!(Level::DEBUG, "list_all_issues");
    let _enter = span.enter();
    debug!("Incoming ListAllIssues");
    let issue_actor = IssueService::from_registry();
    let _ = if let Some(user_id) = with_ctx(|act: &mut WsClient, _| act.user_id.clone()) {
        user_id
    } else {
        return Err(eyre!(
            "Not sure who the user who tried to list all issues is"
        ));
    };
    let resp = issue_actor
        .send(SpanMessage::new(ListAllIssues))
        .await
        .wrap_err("Error handling incoming list all issues")?;

    // Send event back to client with the issues
    match resp {
        Ok(i) => {
            with_ctx(|act: &mut WsClient, ctx| {
                let issues = match i {
                    Some(issues) => issues
                        .into_iter()
                        .map(|issue: services::issue::InternalIssue| Issue {
                            id: Some(issue.id),
                            title: issue.title,
                            description: issue.description,
                            alternatives: issue
                                .alternatives
                                .into_iter()
                                .map(|alt: db::alternative::InternalAlternative| Alternative {
                                    id: Some(alt.id),
                                    title: alt.title,
                                })
                                .collect(),
                            max_voters: Some(issue.max_voters),
                            show_distribution: issue.show_distribution,
                            state: Some(issue.state.to_websocket()),
                            votes: Some(
                                issue
                                    .votes
                                    .into_iter()
                                    .map(|vote: db::vote::InternalVote| OutgoingVote {
                                        id: vote.id,
                                        user_id: vote.user_id,
                                        alternative_id: vote.alternative_id,
                                    })
                                    .collect(),
                            ),
                        })
                        .collect(),
                    None => Vec::new(), // Don't send anything if we don't have any? or send empty?
                };
                act.send_json(ctx, &OutgoingMessage::AllIssues(Issues { issues }))
            })
            .wrap_err("Failed to send all issues to client")?;
            Ok(())
        }

        _ => Ok(()), // how to handle error hmm
    }
}

async fn handle_ws_message(text: String) -> Result<(), Report> {
    let m = serde_json::from_str(&text).wrap_err("JSON decode")?;
    match m {
        IncomingMessage::Vote(vote) => handle_vote(vote).await,
        IncomingMessage::Login(login) => handle_login(login).await,
        IncomingMessage::Reconnect(reconnect) => handle_reconnect(reconnect).await,
        IncomingMessage::CreateIssue(issue) => handle_create_issue(issue).await,
        IncomingMessage::Registration(registration) => handle_registration(registration).await,
        IncomingMessage::ListAllIssues => handle_list_all_issues().await,
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
        service.do_send(SpanMessage::new(connect.clone()));
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
                    ctx.spawn(
                        async move {
                            if let Err(err) = handle_ws_message(text).await {
                                report_error(err);
                            }
                        }
                        .interop_actor_boxed(self),
                    );
                }
                ws::Message::Close(reason) => {
                    debug!("Got close message from WS. Reason: {:#?}", reason);
                    ctx.close(reason)
                }
                ws::Message::Ping(msg) => ctx.pong(&msg),
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
        let res = self.send_json(
            ctx,
            &OutgoingMessage::Issue(Issue {
                id: Some(issue.id),
                title: issue.title,
                description: issue.description,
                state: Some(issue.state.to_websocket()),
                alternatives: issue
                    .alternatives
                    .into_iter()
                    .map(|alt: db::alternative::InternalAlternative| Alternative {
                        id: Some(alt.id),
                        title: alt.title,
                    })
                    .collect(),
                votes: Some(
                    issue
                        .votes
                        .into_iter()
                        .map(|vote: InternalVote| OutgoingVote {
                            id: vote.id,
                            alternative_id: vote.alternative_id,
                            user_id: vote.user_id,
                        })
                        .collect(),
                ),
                max_voters: Some(issue.max_voters),
                show_distribution: issue.show_distribution,
            }),
        );
        if let Err(err) = res {
            report_error(err);
        }
    }
}

impl Handler<services::ListAllIssues> for WsClient {
    type Result = ();

    fn handle(&mut self, msg: services::ListAllIssues, _: &mut Self::Context) {
        debug!("Incoming 'list all issues' event");
        let issues = msg.0;
        debug!("We have this many issues: {}", issues.len());
    }
}

impl Handler<BroadcastVote> for WsClient {
    type Result = ();

    fn handle(&mut self, msg: BroadcastVote, ctx: &mut Self::Context) {
        let vote = msg.0;
        let res = self.send_json(
            ctx,
            &OutgoingMessage::Vote(OutgoingVote {
                id: vote.id,
                alternative_id: vote.alternative_id,
                user_id: vote.user_id,
            }),
        );
        if let Err(err) = res {
            report_error(err);
        }
    }
}
