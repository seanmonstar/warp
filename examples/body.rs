extern crate serde;
#[macro_use] extern crate serde_derive;
extern crate pretty_env_logger;
extern crate warp;

use warp::Filter;

#[derive(Deserialize, Serialize)]
struct Employee {
    name: String,
    rate: u32,
}

fn main() {
    pretty_env_logger::init();

    let promote = warp::path::exact("employees")
        .and(warp::path::<u32>())
        .and(warp::body::json::<Employee>())
        .map(|rate, mut employee: Employee| {
            employee.rate = rate;
            warp::reply::json(employee)
        });

    // POST /employees/:rate  {"name":"Sean","rate":2}
    let routes = warp::post(promote);

    warp::serve(routes)
        .run(([127, 0, 0, 1], 3030));
}
