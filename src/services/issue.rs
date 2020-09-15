use crate::websocket::Issue;
use crate::{
    db::{
        self, alternative::InternalAlternative, issue::InternalIssueState, vote::InternalVote,
        DbExecutor,
    },
    span::{AsyncSpanHandler, SpanMessage},
};
use actix::prelude::*;
use color_eyre::eyre::Report;
use db::issue::IssueId;
use tracing::{debug, info};

#[derive(Clone)]
pub struct InternalIssue {
    pub id: IssueId,
    pub title: String,
    pub description: String,
    pub state: InternalIssueState,
    pub alternatives: Vec<InternalAlternative>,
    pub votes: Vec<InternalVote>,
    pub max_voters: i32,
    pub show_distribution: bool,
}

impl InternalIssue {
    fn from_db(
        issue: db::issue::InternalIssue,
        alternatives: Vec<db::alternative::InternalAlternative>,
        votes: Vec<db::vote::InternalVote>,
    ) -> Self {
        Self {
            id: issue.id,
            title: issue.title,
            description: issue.description,
            state: issue.state,
            max_voters: issue.max_voters,
            show_distribution: issue.show_distribution,
            alternatives,
            votes,
        }
    }
}

pub struct IssueService {}

impl IssueService {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for IssueService {
    fn default() -> Self {
        unimplemented!("Issue actor can't be unitialized using default because it needs a logger")
    }
}

impl Actor for IssueService {
    type Context = Context<Self>;
}

#[derive(Message)]
#[rtype(result = "Result<Option<InternalIssue>, Report>")]
pub struct ActiveIssue;

#[async_trait::async_trait]
impl AsyncSpanHandler<ActiveIssue> for IssueService {
    async fn handle(_msg: ActiveIssue) -> Result<Option<InternalIssue>, Report> {
        info!("Sending active issue");
        let issue: Option<db::issue::InternalIssue> = DbExecutor::from_registry()
            .send(SpanMessage::new(db::issue::ActiveIssue()))
            .await??;
        match issue {
            Some(issue) => {
                debug!(
                    "Issue found, retrieving alternatives and votes {:#?}",
                    id = issue.id
                );
                let (alternatives, votes) = tokio::join!(
                    DbExecutor::from_registry().send(SpanMessage::new(
                        db::alternative::AlternativesForIssueId(issue.id.clone(),)
                    )),
                    DbExecutor::from_registry()
                        .send(SpanMessage::new(db::vote::VotesForIssue(issue.id.clone(),))),
                );
                let alternatives = alternatives??;
                let votes = votes??;
                debug!("Alternatives found {}", alternatives.len());
                debug!("Votes found {}", votes.len());
                Ok(Some(InternalIssue::from_db(issue, alternatives, votes)))
            }
            None => Ok(None),
        }
    }
}
crate::span_message_async_impl!(ActiveIssue, IssueService);

#[derive(Message)]
#[rtype(result = "Result<Option<InternalIssue>, Report>")]
pub struct NewIssue(pub Issue);

#[async_trait::async_trait]
impl AsyncSpanHandler<NewIssue> for IssueService {
    async fn handle(_msg: NewIssue) -> Result<Option<InternalIssue>, Report> {
        info!("Creating new issue");
        let issue: Option<db::issue::InternalIssue> = DbExecutor::from_registry()
            .send(SpanMessage::new(db::issue::NewIssue(_msg.0)))
            .await??;
        debug!("pog back from db stuff phew");
        match issue {
            Some(issue) => {
                debug!("some back from db");
                Ok(Some(InternalIssue {
                    id: issue.id,
                    alternatives: Vec::new(),
                    description: issue.description,
                    title: issue.title,
                    max_voters: issue.max_voters,
                    show_distribution: issue.show_distribution,
                    state: issue.state,
                    votes: Vec::new(),
                }))
            }
            None => {
                debug!("none back from db");
                Ok(None)
            }
        }
    }
}
crate::span_message_async_impl!(NewIssue, IssueService);

impl Supervised for IssueService {}
impl ArbiterService for IssueService {}
