use super::{Connect, Login, Disonnect};
use crate::websocket::WsClient;
use actix::prelude::*;
use slog;
use slog::{debug, info};
use std::collections::HashMap;
use uuid::Uuid;

extern crate slog_term;

// Types

#[derive(Clone, Hash, PartialEq, Eq)]
pub struct UserId(pub String);

// Actor
pub struct ClientActor {
    logger: slog::Logger,
    clients: HashMap<Addr<WsClient>, InternalClient>,
}

impl ClientActor {
    pub fn new(logger: slog::Logger) -> Self {
        ClientActor {
            logger,
            clients: HashMap::new(),
        }
    }
}

impl Default for ClientActor {
    fn default() -> Self {
        unimplemented!(
            "Client actor can't be unitialized using default because it needs a logger"
        )
    }
}

impl Actor for ClientActor {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        info!(self.logger, "Client actor started");
    }
}

impl Handler<Connect> for ClientActor {
    type Result = ();

    // reconnection problem: see if the client exists already? maybe if the client connects and sends Id we can connect them back together somehow?
    // otherwise we'll create a new addr and a new uuid and the client will have to log in again.
    // this is required as we'd like to be able to restart the web socket server for updates while people are connected.

    fn handle(&mut self, msg: Connect, _ctx: &mut Context<Self>) -> Self::Result {
        debug!(self.logger, "Adding new client to ClientActor");

        let uuid = Uuid::new_v4().to_hyphenated().to_string();

        debug!(self.logger, "Adding new client {}", uuid); // @todo: use real slog

        let client = InternalClient{
            username: "".to_owned(), // we don't know this at this point in time
            id: uuid,
        };

        self.clients.insert(msg.addr.to_owned(), client.clone());

        // Tell client its ID
        msg.addr.do_send(IncomingNewClient(client));
    }
}

impl Handler<Disonnect> for ClientActor {
    type Result = ();

    fn handle(&mut self, msg: Disonnect, _ctx: &mut Context<Self>) -> Self::Result {
        debug!(self.logger, "Removing client from ClientActor");
        self.clients.remove(&msg.addr);
    }
}

impl Handler<Login> for ClientActor {
    type Result = ();

    fn handle(&mut self, msg: Login, _ctx: &mut Context<Self>) -> Self::Result {

        debug!(self.logger, "Incoming login in ClientActor");

        // Now we want to connect an existing WsClient to a user by e.g. setting a username
        for (addr, client) in self.clients.clone() {
            // I think we ideally would want this maybe, but I'm not sure how to get Addr in here.
            // if addr != &msg.addr {
            //     continue;
            // }
            if client.id != msg.user_id {
                debug!(self.logger, "Not it {} vs {}", client.id, msg.user_id);
                continue;
            }

            debug!(self.logger, "Found existing client during login woooooohooo");

            let updated_client = InternalClient{
                id: client.id.clone(),
                username: msg.username.clone(),
            };

            self.clients.entry(addr.clone()).or_insert(updated_client.clone());

            // Send updated client details back to client
            addr.do_send(IncomingNewClient(updated_client.clone()));

            // Ask VoteActor to push out existing vote for user if it exists
            // Or maybe do this as a separate message? True message driven
            // let vote_actor = VoteActor::from_registry();
            // vote_actor
            //     .send(MyVote(UserId(username2.to_owned())))
            //     .into_actor(self)
            //     .then(|res, act, _ctx| {
            //         match res {
            //             Ok(vote) => {
            //                 addr.do_send(MyVote(vote))
            //             }
            //             Err(err) =>
            //                 debug!(self.logger, "No current vote for user")
            //         }
            //     })
            //     ;
        }
    }
}

#[derive(Clone)]
pub struct InternalClient {
    pub id: String,
    pub username: String,
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct IncomingNewClient(pub InternalClient);

impl SystemService for ClientActor {}
impl Supervised for ClientActor {}
