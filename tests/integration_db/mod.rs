use dotenv::dotenv;
use lazy_static::lazy_static;
use sqlx::Executor;
use sqlx::{postgres::PgConnectOptions, PgPool};
use std::fs;
use tokio::sync::Mutex;
use tracing::{debug, span};
use vaas_server::db;

lazy_static! {
    static ref CREATE_DB_MUTEX: Mutex<()> = Mutex::new(());
}

async fn create_test_db(pool: PgPool, test_db: &str) {
    let _lock = CREATE_DB_MUTEX.lock().await;
    debug!("Creating new test db");

    sqlx::query(&format!("DROP DATABASE IF EXISTS {}", test_db))
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query(&format!("CREATE DATABASE {}", test_db))
        .execute(&pool)
        .await
        .unwrap();
}

async fn init_test_db(pool: &PgPool) {
    let mut resources: Vec<fs::DirEntry> = fs::read_dir("resources")
        .unwrap()
        .map(|entry| entry.unwrap())
        .collect();
    resources.sort_by_key(|r| r.file_name());
    debug!("Executing init SQL in test db");
    for resource in resources {
        pool.execute(fs::read_to_string(resource.path()).unwrap().as_str())
            .await
            .unwrap();
    }
}

async fn drop_test_db(pool: PgPool, test_db: &str) {
    let _lock = CREATE_DB_MUTEX.lock().await;
    debug!("Dropping test db");
    sqlx::query(&format!("DROP DATABASE {}", test_db))
        .execute(&pool)
        .await
        .unwrap();
}

pub struct IntegrationTestDb {
    db_name: String,
    pool: PgPool,
    template_connect_options: PgConnectOptions,
}

impl IntegrationTestDb {
    pub async fn new() -> Self {
        // TODO: read from some shared config
        dotenv().ok();
        let template_connect_options: PgConnectOptions = std::env::var("DATABASE_URL")
            .expect("DATABASE_URL must be set")
            .parse()
            .unwrap();

        // Creating test database with random name
        let db_name = format!("integration_{}", uuid::Uuid::new_v4().to_simple());
        let span = span!(tracing::Level::DEBUG, "test_db", test_db = db_name.as_str());
        let _enter = span.enter();
        let template_pool = db::new_pool_with(template_connect_options.clone())
            .await
            .unwrap();
        create_test_db(template_pool, &db_name).await;

        let integration_pool = template_connect_options.clone().database(&db_name);
        let pool = db::new_pool_with(integration_pool).await.unwrap();
        init_test_db(&pool).await;

        Self {
            db_name,
            pool,
            template_connect_options,
        }
    }

    pub fn pool(&self) -> PgPool {
        self.pool.clone()
    }
}

impl Drop for IntegrationTestDb {
    fn drop(&mut self) {
        // Cleanup test db after test is finished
        let db_name = self.db_name.clone();
        let template_connect_options = self.template_connect_options.clone();
        // Probably not the right way to run async code in drop, but it works
        tokio::task::spawn_blocking(move || {
            let span = span!(tracing::Level::DEBUG, "test_db", test_db = db_name.as_str());
            let _enter = span.enter();
            actix_rt::System::new("Cleanup").block_on(async move {
                let template_pool = db::new_pool_with(template_connect_options.clone())
                    .await
                    .unwrap();
                drop_test_db(template_pool, &db_name).await;
                debug!("Dropped test db");
            });
        });
    }
}
