#![deny(warnings)]
use warp::Filter;

use std::sync::Arc;

#[derive(Clone)]
struct Pool<T>(Arc<T>);
type DbPool = Pool<&'static str>;
type RedisPool = Pool<i32>;
type BoolPool = Pool<bool>;

impl<T> Pool<T> {
    fn new(data: T) -> Self {
        Pool(Arc::new(data))
    }

    fn to_string(self) -> String
    where
        T: ToString,
    {
        self.0.to_string()
    }
}

#[tokio::main]
async fn main() {
    let index = warp::path::end()
        .and(warp::ext::get::<DbPool>())
        .and(warp::ext::get::<RedisPool>())
        .map(|db: DbPool, redis: RedisPool| {
            format!(
                "db: {db}\nredis: {redis}",
                redis = redis.to_string(),
                db = db.to_string()
            )
        });

    let show_db = warp::path!("db")
        .and(warp::ext::get::<DbPool>())
        .map(|it: DbPool| it.to_string());

    let show_redis = warp::path!("redis")
        .and(warp::ext::get::<RedisPool>())
        .map(|it: RedisPool| it.to_string());

    let show_local = warp::path!("local")
        .and(warp::ext::get::<BoolPool>())
        .map(|it: BoolPool| it.to_string())
        .with(warp::ext::provide(Pool::new(true)));

    let filter = index
        .or(show_db)
        .or(show_redis)
        .or(show_local)
        .with(warp::ext::provide(Pool::new("Db")))
        .with(warp::ext::provide(Pool::new(42)));

    warp::serve(filter).run(([127, 0, 0, 1], 3030)).await;
}
