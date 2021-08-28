#![deny(warnings)]

use bb8::{Pool, RunError};
use bb8_postgres::PostgresConnectionManager;
use std::{convert::Infallible, str::FromStr};
use tokio_postgres::{config::Config, NoTls};
use warp::{reject::Reject, Filter, Rejection};

type ConnectionPool = Pool<PostgresConnectionManager<NoTls>>;

#[derive(Debug)]
struct ConnectionError(RunError<tokio_postgres::Error>);

impl Reject for ConnectionError {}

fn with_pool(
    pool: ConnectionPool,
) -> impl Filter<Extract = (ConnectionPool,), Error = Infallible> + Clone {
    warp::any().map(move || pool.clone())
}

async fn index_handler(pool: ConnectionPool) -> Result<String, Rejection> {
    let connection = pool.get().await.map_err(|e| ConnectionError(e.into()))?;
    let stmt = connection
        .prepare("SELECT 1")
        .await
        .map_err(|e| ConnectionError(e.into()))?;
    let row = connection
        .query_one(&stmt, &[])
        .await
        .map_err(|e| ConnectionError(e.into()))?;
    Ok(row.get::<usize, i32>(0).to_string())
}

#[tokio::main]
async fn main() {
    // The simplest way to start the DB is using Docker:
    // docker run --name postgres -e POSTGRES_PASSWORD=postgres -p 5432:5432 -d postgres
    let config = Config::from_str("postgresql://postgres:postgres@localhost:5432").unwrap();
    let manager = PostgresConnectionManager::new(config, NoTls);
    let pool = match Pool::builder().build(manager).await {
        Ok(pool) => pool,
        Err(e) => panic!("Pool builder error: {:?}", e),
    };
    let routes = warp::any()
        .and(with_pool(pool.clone()))
        .and_then(index_handler);
    warp::serve(routes).run(([127, 0, 0, 1], 3030)).await;
}
