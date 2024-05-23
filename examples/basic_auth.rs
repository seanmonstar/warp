//! Example showing howto use an async lock guarded user-db with HTTP basic authentication
use headers::authorization::Basic;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use warp::Filter;

#[derive(Debug)]
enum AuthRejections {
    InvalidPassword { username: String },
    StalePassword { username: String },
    UnknownUser { username: String },
}

impl warp::reject::Reject for AuthRejections {}

/// here we verify the user provided the correct password and his account is still active
/// since we are modifying the contents of the hashmap with our user-accounts we need to
/// guard it with a Mutex
async fn is_authenticated(
    auth_header: Basic,
    pwdb: Arc<Mutex<HashMap<String, UserCredential>>>,
) -> Result<String, warp::Rejection> {
    let username = auth_header.username().to_string();
    let mut pwdb = pwdb.lock().await;
    if let Some(cred) = pwdb.get_mut(&username) {
        if cred.time_to_live == 0 {
            return Err(warp::reject::custom(AuthRejections::StalePassword {
                username,
            }));
        }
        cred.time_to_live = cred.time_to_live.saturating_sub(1);
        if cred.password == auth_header.password() {
            Ok(username)
        } else {
            Err(warp::reject::custom(AuthRejections::InvalidPassword {
                username,
            }))
        }
    } else {
        Err(warp::reject::custom(AuthRejections::UnknownUser {
            username,
        }))
    }
}

/// Stores users password in cleartext, not suggested practice for production!
/// time_to_live has to be > 0 to mark the account as active
struct UserCredential {
    password: String,
    time_to_live: u64,
}

impl UserCredential {
    pub fn new(password: &str, time_to_live: u64) -> Self {
        Self {
            password: password.to_string(),
            time_to_live,
        }
    }
}

#[tokio::main]
async fn main() {
    let pwdb: HashMap<_, _> = vec![
        ("alice".to_string(), UserCredential::new("wonderland", 0)),
        ("bob".to_string(), UserCredential::new("cat", 10)),
        ("carl".to_string(), UserCredential::new("IePai4ph", 100)),
    ]
    .into_iter()
    .collect();
    let pwdb = Arc::new(Mutex::new(pwdb));
    let pwdb = warp::any().map(move || pwdb.clone());
    let routes = warp::auth::basic("Realm name")
        .and(pwdb.clone())
        .and_then(is_authenticated)
        .map(|user: String| format!("Hello, {} you're authenticated", user))
        .recover(handle_rejections);

    warp::serve(routes).run(([127, 0, 0, 1], 3030)).await;
}

async fn handle_rejections(
    rej: warp::reject::Rejection,
) -> Result<warp::reply::Response, std::convert::Infallible> {
    let res = if let Some(inner) = rej.find::<AuthRejections>() {
        match inner {
            AuthRejections::InvalidPassword { .. } => {
                warp_response("", "text/plain", warp::http::status::StatusCode::FORBIDDEN)
            }
            AuthRejections::StalePassword { username } => warp_response(
                &format!(
                    r#"{{ "reason": "Password Expired", "username": "{}" }}"#,
                    username
                ),
                "application/json",
                warp::http::status::StatusCode::FORBIDDEN,
            ),
            AuthRejections::UnknownUser { .. } => {
                warp_response("", "text/plain", warp::http::status::StatusCode::FORBIDDEN)
            }
        }
    } else {
        // TODO: wait for issue #451 to be resolved
        //rej.into_response()
        // this breaks the authentication because we do not handle the warp internal Rejections
        // correctly here returning statuscode 500 when we shouldn't.
        println!("Missing implementation to handle {:?}", rej);
        warp_response(
            "",
            "text/plain",
            warp::http::status::StatusCode::INTERNAL_SERVER_ERROR,
        )
    };
    Ok(res)
}

fn warp_response(
    body: &str,
    content_type: &str,
    status: warp::http::status::StatusCode,
) -> warp::reply::Response {
    let mut r = warp::reply::Response::new(body.to_string().into());
    *r.status_mut() = status;
    r.headers_mut().insert(
        "Content-Type",
        warp::http::HeaderValue::from_str(content_type).unwrap(),
    );
    r
}
