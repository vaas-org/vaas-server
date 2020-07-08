use super::broadcast::BroadcastActor;
use crate::span::{AsyncSpanHandler, SpanMessage};
use crate::{
    async_message_handler_with_span,
    db::{self, alternative::AlternativeId, user::UserId, vote::InternalVote, DbExecutor},
};
use actix::prelude::*;
use color_eyre::eyre::Report;
use tracing::{debug, info};

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
