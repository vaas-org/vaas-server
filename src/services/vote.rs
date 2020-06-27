use super::broadcast::BroadcastActor;
use crate::managers::{
    alternative::AlternativeId,
    user::UserId,
    vote::{InternalVote, VoteId},
};
use actix::prelude::*;
use std::collections::HashMap;
use tracing::{debug, info};

// IncomingGetMyVote could have Addr<WsClient> arg
// so that it can respond to messages.. maybe?
#[derive(Message)]
#[rtype(result = "()")]
pub struct IncomingGetMyVote(pub UserId);

#[derive(Message)]
#[rtype(result = "()")]
pub struct IncomingVoteMessage(pub UserId, pub AlternativeId);

#[derive(Message, Clone)]
#[rtype(result = "()")]
pub struct BroadcastVote(pub InternalVote);

// Actor

pub struct VoteActor {
    votes: HashMap<AlternativeId, Vec<InternalVote>>,
}

impl VoteActor {
    pub fn new() -> Self {
        Self {
            votes: HashMap::new(),
        }
    }

    fn votes_for_alternative(&mut self, alternative_id: AlternativeId) -> &mut Vec<InternalVote> {
        self.votes.entry(alternative_id).or_insert_with(Vec::new)
    }

    pub fn add_vote(&mut self, alternative_id: AlternativeId, user_id: UserId) {
        let votes = self.votes_for_alternative(alternative_id.clone());
        let vote = InternalVote {
            id: VoteId::new(),
            alternative_id,
            user_id,
        };
        votes.push(vote.clone());

        let broadcast = BroadcastActor::from_registry();
        broadcast.do_send(BroadcastVote(vote));
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
        info!("Vote actor started");
    }
}

#[derive(Message)]
#[rtype(result = "InternalVote")]
pub struct MyVote(pub UserId);

// impl Handler<MyVote> for VoteActor {
//     type Result = MessageResult<MyVote>;

//     fn handle(&mut self, msg: MyVote, _ctx: &mut Context<Self>) -> Self::Result {
//         debug!("received new client login in VoteActor");
//         let MyVote(user_id) = msg;
//         let vote;
//         for (_, alt) in self.votes.clone() {
//             for v in alt {
//                 if v.user_id == user_id {
//                     debug!("---👀 got existing vote for user");

//                     vote = v;
//                 }
//             }
//         }
//         return MessageResult(vote);
//     }
// }

impl Handler<IncomingVoteMessage> for VoteActor {
    type Result = ();
    fn handle(&mut self, msg: IncomingVoteMessage, _ctx: &mut Context<Self>) -> Self::Result {
        debug!("VoteActor handling IncomingVoteMessage");
        let IncomingVoteMessage(user_id, alternative_id) = msg;
        self.add_vote(alternative_id, user_id);
    }
}

impl SystemService for VoteActor {}
impl Supervised for VoteActor {}

#[cfg(test)]
mod tests {
    use super::*;

    #[actix_rt::test]
    async fn add_vote() {
        let mut service = VoteActor::new();
        let alternative = AlternativeId::new();
        let user = UserId::new();

        let votes = service.votes_for_alternative(alternative.clone());
        let votes = votes.clone();
        assert_eq!(votes, []);

        service.add_vote(alternative.clone(), user.clone());
        let votes = service.votes_for_alternative(alternative.clone());
        assert_eq!(votes.len(), 1);
        assert_eq!(votes[0].user_id, user);
        assert_eq!(votes[0].alternative_id, alternative);
    }
}
