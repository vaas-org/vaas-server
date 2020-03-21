use actix::prelude::*;

#[derive(Clone)]
pub enum InternalIssueState {
    NotStarted,
    InProgress,
    Finished,
}

#[derive(Clone)]
pub struct InternalAlternative {
    pub id: String,
    pub title: String,
}

#[derive(Clone)]
pub struct InternalVote {
    pub id: String,
    pub alternative_id: String,
}

#[derive(Clone)]
pub struct InternalIssue {
    pub id: String,
    pub title: String,
    pub description: String,
    pub state: InternalIssueState,
    pub alternatives: Vec<InternalAlternative>,
    pub votes: Vec<InternalVote>,
    pub max_voters: u32,
    pub show_distribution: bool,
}

pub struct IssueService {
    issue: InternalIssue,
}

impl IssueService {
    pub fn mocked() -> IssueService {
        IssueService {
            issue: InternalIssue {
                id: "0".to_owned(),
                title: "coronvorus bad??".to_owned(),
                description: "yes or yes".to_owned(),
                state: InternalIssueState::InProgress,
                alternatives: vec![
                    InternalAlternative {
                        id: "1".to_owned(),
                        title: "yes".to_owned(),
                    },
                    InternalAlternative {
                        id: "2".to_owned(),
                        title: "other yes".to_owned(),
                    },
                    InternalAlternative {
                        id: "3".to_owned(),
                        title: "my name is trump i have control".to_owned(),
                    },
                ],
                votes: vec![],
                max_voters: 10,
                show_distribution: true,
            },
        }
    }
}

impl Actor for IssueService {
    type Context = Context<Self>;
}

#[derive(Message)]
#[rtype(result = "InternalIssue")]
pub struct ActiveIssue;

impl Handler<ActiveIssue> for IssueService {
    type Result = MessageResult<ActiveIssue>;

    fn handle(&mut self, _msg: ActiveIssue, _ctx: &mut Context<Self>) -> Self::Result {
        MessageResult(self.issue.clone())
    }
}
