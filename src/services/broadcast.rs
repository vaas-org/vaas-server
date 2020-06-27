use super::{vote::BroadcastVote, Connect, Disconnect};
use crate::websocket::WsClient;
use actix::prelude::*;
use std::collections::HashSet;
use tracing::{debug, info};

// Actor
pub struct BroadcastActor {
    clients: HashSet<Addr<WsClient>>,
}

impl BroadcastActor {
    pub fn new() -> Self {
        BroadcastActor {
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
        info!("Broadcast actor started");
    }
}

impl Handler<Connect> for BroadcastActor {
    type Result = <Connect as Message>::Result;

    fn handle(&mut self, msg: Connect, _ctx: &mut Context<Self>) -> Self::Result {
        debug!("Adding new client to broadcast");
        self.clients.insert(msg.addr);
        Ok(())
    }
}

impl Handler<Disconnect> for BroadcastActor {
    type Result = ();

    fn handle(&mut self, msg: Disconnect, _ctx: &mut Context<Self>) -> Self::Result {
        debug!("Removing client from broadcast");
        self.clients.remove(&msg.addr);
    }
}

macro_rules! broadcast_handler {
    ($message_type:ident) => {
        impl Handler<$message_type> for BroadcastActor  {
            type Result = ();

            fn handle(&mut self, msg: $message_type, _ctx: &mut Context<Self>) -> Self::Result {
                debug!(
                    "Broadcasting {type} to clients. Number of clients: {clients}",
                    type = stringify!($message_type),
                    clients = self.clients.len()
                );
                for client in &self.clients {
                    client.do_send(msg.clone());
                }
            }
        }
    };
}

broadcast_handler!(BroadcastVote);

impl SystemService for BroadcastActor {}
impl Supervised for BroadcastActor {}
