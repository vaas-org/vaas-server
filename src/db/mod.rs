pub mod alternative;
pub mod issue;
pub mod session;
pub mod user;
pub mod vote;

use actix::prelude::*;
use sqlx::{postgres::PgConnectOptions, PgPool};

#[derive(Debug)]
pub struct DbExecutor(pub PgPool);

impl DbExecutor {
    pub fn pool(&mut self) -> PgPool {
        self.0.clone()
    }
}

impl Actor for DbExecutor {
    type Context = Context<Self>;
}

impl Default for DbExecutor {
    fn default() -> Self {
        unimplemented!("DbExecutor cannot automatically be started");
    }
}

impl SystemService for DbExecutor {}
impl Supervised for DbExecutor {}

pub async fn new_pool(database_url: &str) -> Result<PgPool, sqlx::Error> {
    let connect_options: PgConnectOptions = database_url
        .parse()
        .map_err(sqlx::Error::ParseConnectOptions)?;
    new_pool_with(connect_options).await
}

pub async fn new_pool_with(connect_options: PgConnectOptions) -> Result<PgPool, sqlx::Error> {
    PgPool::builder()
        .max_size(5) // maximum number of connections in the pool
        .build_with(connect_options)
        .await
}
