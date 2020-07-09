use dotenv::dotenv;
use lazy_static::lazy_static;
use sqlx::{postgres::PgConnectOptions, PgPool};
use tokio::sync::Mutex;
use tracing::info;
use vaas_server::db;

lazy_static! {
    static ref CREATE_DB_MUTEX: Mutex<()> = Mutex::new(());
}

async fn create_test_db(pool: PgPool, test_db: &str) {
    let _lock = CREATE_DB_MUTEX.lock().await;

    sqlx::query(&format!("DROP DATABASE IF EXISTS {}", test_db))
        .execute(&pool)
        .await
        .unwrap();
    let current = sqlx::query!("SELECT current_database()")
        .fetch_one(&pool)
        .await
        .unwrap()
        .current_database
        .unwrap();
    sqlx::query(&format!("CREATE DATABASE {} TEMPLATE {}", test_db, current))
        .execute(&pool)
        .await
        .unwrap();
}

async fn drop_test_db(pool: PgPool, test_db: &str) {
    let _lock = CREATE_DB_MUTEX.lock().await;
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
        // Creating test database with random name by copying original
        // TODO: should probably create this from scratch by using the table schemas instead
        let db_name = format!("integration_{}", uuid::Uuid::new_v4().to_simple());
        let template_pool = db::new_pool_with(template_connect_options.clone())
            .await
            .unwrap();
        create_test_db(template_pool, &db_name).await;

        let integration_pool = template_connect_options.clone().database(&db_name);
        let pool = db::new_pool_with(integration_pool).await.unwrap();

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
            actix_rt::System::new("Cleanup").block_on(async move {
                let template_pool = db::new_pool_with(template_connect_options.clone())
                    .await
                    .unwrap();
                drop_test_db(template_pool, &db_name).await;
                info!(name = db_name.as_str(), "Dropped test db");
            });
        });
    }
}
