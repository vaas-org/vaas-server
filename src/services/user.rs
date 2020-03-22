use super::vote::{VoteActor, IncomingGetMyVote};
use actix::prelude::*;
use slog::debug;
use slog::info;
use std::collections::HashMap;
use uuid::Uuid;

// Types

#[derive(Clone, Hash, PartialEq, Eq)]
pub struct UserId(pub String);

#[derive(Clone)]
pub struct InternalUser {
    pub id: UserId,
    pub username: String,
}

// Messages

#[derive(Message)]
#[rtype(result = "()")]
pub struct IncomingLoginMessage(pub UserId, pub String);

// Actor

pub struct UserActor {
    logger: slog::Logger,
    users: HashMap<String, InternalUser>,
}

impl UserActor {
    pub fn new(logger: slog::Logger) -> Self {
        UserActor {
            logger,
            users: HashMap::new(),
        }
    }
}

impl Default for UserActor {
    fn default() -> Self {
        unimplemented!("User actor can't be unitialized using default because it needs a logger")
    }
}

impl Actor for UserActor {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        info!(self.logger, "User actor started");
    }
}

impl Handler<IncomingLoginMessage> for UserActor {
    type Result = ();
    fn handle(&mut self, msg: IncomingLoginMessage, _ctx: &mut Context<Self>) -> Self::Result {
        debug!(self.logger, "UserActor handling IncomingLoginMessage");
        let IncomingLoginMessage(id, username) = msg;
        let user = self.users.entry(username.clone()).or_insert_with(||InternalUser {
            id: UserId(Uuid::new_v4().to_hyphenated().to_string()),
            username: username.clone(),
        });

        // reference borrows is fun
        // debug!(self.logger, "{}", format!("users {:?}", self.users));

        // @todo: Should be user_id but hacking it for username from frontend currently
        let ref username2 = user.username;

        // Ask VoteActor to push out existing vote for user if it exists
        let vote_actor = VoteActor::from_registry();
        vote_actor.do_send(IncomingGetMyVote(UserId(username2.to_owned())));
    }
}

impl SystemService for UserActor {}
impl Supervised for UserActor {}
