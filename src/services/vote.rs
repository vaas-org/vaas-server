use super::broadcast::BroadcastActor;
use crate::span::{AsyncSpanHandler, SpanMessage};
use crate::{
    async_message_handler_with_span,
    db::{self, alternative::AlternativeId, user::UserId, vote::InternalVote, DbExecutor},
};
use actix::prelude::*;
use color_eyre::eyre::Report;
use tracing::{debug, info};

// IncomingGetMyVote could have Addr<WsClient> arg
// so that it can respond to messages.. maybe?
#[derive(Message)]
#[rtype(result = "()")]
pub struct IncomingGetMyVote(pub UserId);

#[derive(Message)]
#[rtype(result = "Result<(), Report>")]
pub struct IncomingVoteMessage(pub UserId, pub AlternativeId);

#[derive(Message, Clone)]
#[rtype(result = "()")]
pub struct BroadcastVote(pub InternalVote);

// Actor

pub struct VoteActor {}

impl VoteActor {
    pub fn new() -> Self {
        Self {}
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
async_message_handler_with_span!({
    impl AsyncSpanHandler<IncomingVoteMessage> for VoteActor {
        async fn handle(msg: IncomingVoteMessage) -> Result<(), Report> {
            debug!("VoteActor handling IncomingVoteMessage");
            let IncomingVoteMessage(user_id, alternative_id) = msg;

            let vote = DbExecutor::from_registry()
                .send(SpanMessage::new(db::vote::AddVote(user_id, alternative_id)))
                .await??;

            let broadcast = BroadcastActor::from_registry();
            broadcast.do_send(BroadcastVote(vote));
            Ok(())
        }
    }
});

impl SystemService for VoteActor {}
impl Supervised for VoteActor {}
