use crate::async_message_handler_with_span;
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
    ) -> Self {
        Self {
            id: issue.id,
            title: issue.title,
            description: issue.description,
            state: issue.state,
            max_voters: issue.max_voters,
            show_distribution: issue.show_distribution,
            alternatives,
            votes: vec![],
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

async_message_handler_with_span!({
    impl AsyncSpanHandler<ActiveIssue> for IssueService {
        async fn handle(_msg: ActiveIssue) -> Result<Option<InternalIssue>, Report> {
            info!("Sending active issue");
            let issue: Option<db::issue::InternalIssue> = DbExecutor::from_registry()
                .send(SpanMessage::new(db::issue::ActiveIssue()))
                .await??;
            match issue {
                Some(issue) => {
                    debug!("Issue found, retrieving alternatives {:#?}", id = issue.id);
                    let alternatives: Vec<InternalAlternative> = DbExecutor::from_registry()
                        .send(SpanMessage::new(db::alternative::AlternativesForIssueId(
                            issue.id.clone(),
                        )))
                        .await??;
                    debug!("Alternatives found {}", alternatives.len());
                    Ok(Some(InternalIssue::from_db(issue, alternatives)))
                }
                None => Ok(None),
            }
        }
    }
});

impl Supervised for IssueService {}
impl ArbiterService for IssueService {}
