extern crate pretty_env_logger;
extern crate serde;
#[macro_use] extern crate serde_derive;
extern crate warp;

use std::sync::{Arc, Mutex};
use warp::Filter;

#[derive(Deserialize, Serialize)]
struct Todo {
    id: u64,
    text: String,
    completed: bool,
}

/// Provides a RESTful web server managing some Todos.
///
/// API will be:
///
/// - `GET /todos`: return a JSON list of Todos.
/// - `PUT /todos`: create a new Todo.
/// - `POST /todos/:id`: update a specific Todo.
/// - `DELETE /todos/:id`: delete a specific Todo.
fn main() {
    pretty_env_logger::init();

    // So we don't have to tackle how different database work, we'll just use
    // a simple in-memory DB, a vector synchronized by a mutex.
    let db = Arc::new(Mutex::new(Vec::<Todo>::new()));

    // These are some `Filter`s that several of the endpoints share,
    // so we'll define them here and reuse them below...

    // Just the path segment "todos"...
    let todos = warp::path("todos");

    // Combined with `index`, this means nothing comes after "todos".
    // So, for example: `GET /todos`, but not `GET /todos/32`.
    let todos_index = todos.and(warp::path::index());

    // Combined with an id path parameter, for refering to a specific Todo.
    // For example, `POST /todos/32`, but not `POST /todos/32/something-more`.
    let todos_id = todos
        .and(warp::path::param::<u64>())
        .and(warp::path::index());

    // Next, we'll define each our 4 endpoints:

    // `GET /todos`
    let list = {
        // Our handler needs a clone of the database pointer...
        let db = db.clone();

        // Just return a JSON array of all Todos.
        warp::get(todos_index.map(move || {
            warp::reply::json(&*db.lock().unwrap())
        }))
    };

    // `PUT /todos`
    let create = {
        // Our handler needs a clone of the database pointer...
        let db = db.clone();

        // With the path and a JSON body, insert into our database
        // and return `201 Created`.
        let handler = todos_index
            .and(warp::body::json())
            .and_then(move |create: Todo| {
                let mut vec = db
                    .lock()
                    .unwrap();

                for todo in vec.iter() {
                    if todo.id == create.id {
                        // Todo with id already exists, return `400 BadRequest`.
                        return Err(warp::reject::bad_request());
                    }
                }

                // No existing Todo with id, so insert and return `201 Created`.
                vec.push(create);

                Ok(warp::http::StatusCode::CREATED)
            });

        // Only for PUT requests
        warp::put(handler)
    };

    // `POST /todos/:id`
    let update = {
        // Our handler needs a clone of the database pointer...
        let db = db.clone();

        // With the id and a JSON body, try to update an existing Todo,
        // and on success, return `200 OK`, otherwise `404 Not Found`.
        let handler = todos_id
            .and(warp::body::json())
            .and_then(move |id: u64, update: Todo| {
                let mut vec = db
                    .lock()
                    .unwrap();

                // Look for the specified Todo...
                for todo in vec.iter_mut() {
                    if todo.id == id {
                        *todo = update;
                        return Ok(warp::reply());
                    }
                }

                // If the for loop didn't return OK, then the ID doesn't exist...
                Err(warp::reject::not_found())
            });

        // Only for POST requests
        warp::post(handler)
    };

    // `DELETE /todos/:id`
    let delete = {
        // Our handler needs a clone of the database pointer...
        let db = db.clone();

        // With the Todo id, try to remove the specific Todo.
        let handler = todos_id.and_then(move |id: u64| {
            let mut vec = db
                .lock()
                .unwrap();

            let len = vec.len();
            vec.retain(|todo| {
                // Retain all Todos that aren't this id...
                // In other words, remove all that *are* this id...
                todo.id != id
            });

            // If the vec is smaller, we found and deleted a Todo!
            let deleted = vec.len() != len;

            if deleted {
                // respond with a `204 No Content`, which means successful,
                // yet no body expected...
                Ok(warp::http::StatusCode::NO_CONTENT)
            } else {
                // Reject this request with a `404 Not Found`...
                Err(warp::reject::not_found())
            }
        });

        // Only for DELETE requests
        warp::delete(handler)
    };


    // Combine our endpoints, since we want requests to match any of them:
    let api = list.or(create).or(update).or(delete);

    // View access logs by setting `RUST_LOG=todos`.
    let routes = warp::log("todos").decorate(api);

    // Start up the server...
    warp::serve(routes)
        .run(([127, 0, 0, 1], 3030));
}
