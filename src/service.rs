use actix::prelude::*;

#[derive(Message)]
#[rtype(result = "()")]
pub struct ActiveIssue(pub Issue);

#[derive(Message)]
#[rtype(result = "()")]
pub struct Connect {
    pub addr: Recipient<ActiveIssue>,
}

pub enum InternalIssueState {
    NotStarted,
    InProgress,
    Finished,
}

pub struct InternalAlternative {
    pub id: String,
    pub title: String,
}

pub struct InternalVote {
    pub id: String,
    pub alternative_id: String,
}

pub struct Issue {
    pub id: String,
    pub title: String,
    pub description: String,
    pub state: InternalIssueState,
    pub alternatives: Vec<InternalAlternative>,
    pub votes: Vec<InternalVote>,
    pub max_voters: u32,
    pub show_distribution: bool,
}

#[derive(Default)]
pub struct Service;

impl Actor for Service {
    type Context = Context<Self>;
}

impl Handler<Connect> for Service {
    type Result = ();

    fn handle(&mut self, msg: Connect, _ctx: &mut Context<Self>) {
        println!("Handling connect");
        let send_result = msg.addr.do_send(ActiveIssue(Issue {
            id: "0".to_string(),
            title: "coronvorus bad??".to_string(),
            description: "yes or yes".to_string(),
            state: InternalIssueState::InProgress,
            alternatives: vec![
                InternalAlternative {
                    id: "1".to_string(),
                    title: "yes".to_string(),
                },
                InternalAlternative {
                    id: "2".to_string(),
                    title: "other yes".to_string(),
                },
                InternalAlternative {
                    id: "3".to_string(),
                    title: "my name is trump i have control".to_string(),
                },
            ],
            votes: vec![],
            max_voters: 10,
            show_distribution: true,
        }));
        if let Err(err) = send_result {
            eprintln!("Done goofed while sending issue: {:#?}", err);
        }
    }
}
