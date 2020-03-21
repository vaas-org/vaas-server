use super::broadcast::BroadcastActor;
use actix::prelude::*;
use slog::debug;
use slog::info;
use std::collections::HashMap;
use uuid::Uuid;

// Types

#[derive(Clone)]
pub struct UserId(pub String);
#[derive(Clone)]
pub struct VoteId(pub String);
#[derive(Clone, Hash, PartialEq, Eq)]
pub struct AlternativeId(pub String);

#[derive(Clone)]
pub struct InternalVote {
    pub id: VoteId,
    pub alternative_id: AlternativeId,
    pub user_id: UserId,
}

// Messages

#[derive(Message)]
#[rtype(result = "()")]
pub struct IncomingVoteMessage(pub UserId, pub AlternativeId);

#[derive(Message, Clone)]
#[rtype(result = "()")]
pub struct BroadcastVote(pub InternalVote);

// Actor

pub struct VoteActor {
    logger: slog::Logger,
    votes: HashMap<AlternativeId, Vec<InternalVote>>,
}

impl VoteActor {
    pub fn new(logger: slog::Logger) -> Self {
        VoteActor {
            logger,
            votes: HashMap::new(),
        }
    }
}

impl Default for VoteActor {
    fn default() -> Self {
        unimplemented!("Vote actor can't be unitialized using default because it needs a logger")
    }
}

impl Actor for VoteActor {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        info!(self.logger, "Vote actor started");
    }
}

impl Handler<IncomingVoteMessage> for VoteActor {
    type Result = ();
    fn handle(&mut self, msg: IncomingVoteMessage, _ctx: &mut Context<Self>) -> Self::Result {
        debug!(self.logger, "VoteActor handling IncomingVoteMessage");
        let IncomingVoteMessage(user_id, alternative_id) = msg;
        let votes = self.votes.entry(alternative_id.clone()).or_insert(vec![]);
        let vote = InternalVote {
            id: VoteId(Uuid::new_v4().to_hyphenated().to_string()),
            alternative_id,
            user_id,
        };
        votes.push(vote.clone());

        let broadcast = BroadcastActor::from_registry();
        broadcast.do_send(BroadcastVote(vote));
    }
}

impl SystemService for VoteActor {}
impl Supervised for VoteActor {}
