use super::vote::BroadcastVote;
use super::{Connect, Disonnect};
use crate::websocket::WsClient;
use actix::prelude::*;
use slog;
use slog::{debug, info};
use std::collections::HashSet;

// Messages

// Actor
pub struct BroadcastActor {
    logger: slog::Logger,
    clients: HashSet<Addr<WsClient>>,
}

impl BroadcastActor {
    pub fn new(logger: slog::Logger) -> Self {
        BroadcastActor {
            logger,
            clients: HashSet::new(),
        }
    }
}

impl Default for BroadcastActor {
    fn default() -> Self {
        unimplemented!(
            "Broadcast actor can't be unitialized using default because it needs a logger"
        )
    }
}

impl Actor for BroadcastActor {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        info!(self.logger, "Broadcast actor started");
    }
}

impl Handler<Connect> for BroadcastActor {
    type Result = ();

    fn handle(&mut self, msg: Connect, _ctx: &mut Context<Self>) -> Self::Result {
        debug!(self.logger, "Adding new client to broadcast");
        self.clients.insert(msg.addr);
    }
}

impl Handler<Disonnect> for BroadcastActor {
    type Result = ();

    fn handle(&mut self, msg: Disonnect, _ctx: &mut Context<Self>) -> Self::Result {
        debug!(self.logger, "Removing client from broadcast");
        self.clients.remove(&msg.addr);
    }
}

impl Handler<BroadcastVote> for BroadcastActor {
    type Result = ();

    fn handle(&mut self, msg: BroadcastVote, _ctx: &mut Context<Self>) -> Self::Result {
        debug!(
            self.logger,
            "Broadcasting vote to clients. Number of clients: {clients}",
            clients = self.clients.len()
        );
        for client in &self.clients {
            client.do_send(msg.clone());
        }
    }
}
impl SystemService for BroadcastActor {}
impl Supervised for BroadcastActor {}
