use async_graphql::dataloader::DataLoader;
use async_graphql::http::GraphiQLSource;
use async_graphql::{EmptyMutation, EmptySubscription, Schema};
use async_graphql_axum::GraphQL;
use axum::{
    response::{self, IntoResponse},
    routing::get,
    Router,
};
use graphql::title::TitleLoader;
use graphql::Query;
use index::get_index;
use sqlx::sqlite::{
    SqliteAutoVacuum, SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous,
};
use std::env;
use std::error::Error;
use std::{str::FromStr, time::Duration};
use sync::sync_data;
use tokio::net::TcpListener;
use tokio::signal;
use tracing::info;

mod graphql;
mod id;
mod index;
mod kind;
mod sync;

async fn graphiql() -> impl IntoResponse {
    response::Html(GraphiQLSource::build().endpoint("/").finish())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt::init();
    dotenv::dotenv().ok();

    let data_dir =
        std::path::PathBuf::from(std::env::var("DATA_DIR").expect("DATA_DIR must be set"));

    // ensure data dir exists
    std::fs::create_dir_all(&data_dir).map_err(|e| {
        format!(
            "Failed to create data directory '{}': {}. \
            If using host mounts, ensure the directory exists and has correct permissions: \
            mkdir -p {} && chown -R 65532:65532 {}",
            data_dir.display(),
            e,
            data_dir.display(),
            data_dir.display()
        )
    })?;

    let pool = {
        let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
        let pool = SqlitePoolOptions::new()
            .max_connections(2) // avoid locking
            .acquire_timeout(Duration::from_secs(30))
            .connect_with(
                // https://briandouglas.ie/sqlite-defaults/
                SqliteConnectOptions::from_str(&database_url)
                    .expect("Failed to parse SQLite path")
                    .journal_mode(SqliteJournalMode::Wal)
                    .synchronous(SqliteSynchronous::Normal)
                    .busy_timeout(Duration::from_secs(5))
                    .foreign_keys(true)
                    .auto_vacuum(SqliteAutoVacuum::Incremental)
                    .pragma("cache_size", "-20000")
                    .pragma("temp_store", "MEMORY")
                    .pragma("mmap_size", "2147483648")
                    .page_size(8192),
            )
            .await
            .expect("Failed to connect to SQLite");

        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .expect("Failed to run migrations");

        pool
    };

    let index = get_index(&data_dir).unwrap();

    let pool_clone = pool.clone();
    let index_clone = index.clone();
    tokio::spawn(async move {
        loop {
            sync_data(&data_dir, &index_clone, &pool_clone)
                .await
                .unwrap();
            tokio::time::sleep(Duration::from_secs(14400)).await; // 4 hours
        }
    });

    let reader = index.reader()?;
    let schema = Schema::build(Query, EmptyMutation, EmptySubscription)
        .data(DataLoader::new(
            TitleLoader::new(pool.clone()),
            tokio::spawn,
        ))
        .data(pool)
        .data(index)
        .data(reader)
        .finish();

    let app = Router::new().route("/", get(graphiql).post_service(GraphQL::new(schema)));

    let bind_host = env::var("HUBBLE_HOST").unwrap_or("0.0.0.0".to_string());
    let bind_port = env::var("HUBBLE_HOST").unwrap_or("8000".to_string());
    let bind_addr = format!("{}:{}", bind_host, bind_port);
    let listener = TcpListener::bind(bind_addr).await.unwrap();
    info!("Listening on http://{}", listener.local_addr().unwrap());
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();

    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}
