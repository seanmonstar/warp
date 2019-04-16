extern crate futures;
extern crate pretty_env_logger;
extern crate warp;

use futures::{
    future::poll_fn,
    sync::{mpsc, oneshot},
    Future, Stream,
};
use std::collections::HashMap;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, Mutex,
};
use warp::{sse::ServerSentEvent, Buf, Filter};

/// Our global unique user id counter.
static NEXT_USER_ID: AtomicUsize = AtomicUsize::new(1);

/// Message variants.
enum Message {
    UserId(usize),
    Reply(String),
}

/// Our state of currently connected users.
///
/// - Key is their id
/// - Value is a sender of `Message`
type Users = Arc<Mutex<HashMap<usize, mpsc::UnboundedSender<Message>>>>;

fn main() {
    pretty_env_logger::init();

    // Keep track of all connected users, key is usize, value
    // is a event stream sender.
    let users = Arc::new(Mutex::new(HashMap::new()));
    // Turn our "state" into a new Filter...
    let users = warp::any().map(move || users.clone());

    // POST /chat -> send message
    let chat_send = warp::path("chat")
        .and(warp::post2())
        .and(warp::path::param::<usize>())
        .and(warp::body::content_length_limit(500))
        .and(warp::body::concat().and_then(|body: warp::body::FullBody| {
            std::str::from_utf8(body.bytes())
                .map(String::from)
                .map_err(warp::reject::custom)
        }))
        .and(users.clone())
        .map(|my_id, msg, users| {
            user_message(my_id, msg, &users);
            warp::reply()
        });

    // GET /chat -> messages stream
    let chat_recv =
        warp::path("chat")
            .and(warp::sse())
            .and(users)
            .map(|sse: warp::sse::Sse, users| {
                // reply using server-sent events
                let stream = user_connected(users);
                sse.reply(warp::sse::keep_alive().stream(stream))
            });

    // GET / -> index html
    let index = warp::path::end().map(|| {
        warp::http::Response::builder()
            .header("content-type", "text/html; charset=utf-8")
            .body(INDEX_HTML)
    });

    let routes = index.or(chat_recv).or(chat_send);

    warp::serve(routes).run(([127, 0, 0, 1], 3030));
}

fn user_connected(
    users: Users,
) -> impl Stream<Item = impl ServerSentEvent + Send + 'static, Error = warp::Error> + Send + 'static
{
    // Use a counter to assign a new unique ID for this user.
    let my_id = NEXT_USER_ID.fetch_add(1, Ordering::Relaxed);

    eprintln!("new chat user: {}", my_id);

    // Use an unbounded channel to handle buffering and flushing of messages
    // to the event source...
    let (tx, rx) = mpsc::unbounded();

    match tx.unbounded_send(Message::UserId(my_id)) {
        Ok(()) => (),
        Err(_disconnected) => {
            // The tx is disconnected, our `user_disconnected` code
            // should be happening in another task, nothing more to
            // do here.
        }
    }

    // Make an extra clone of users list to give to our disconnection handler...
    let users2 = users.clone();

    // Save the sender in our list of connected users.
    users.lock().unwrap().insert(my_id, tx);

    // Create channel to track disconnecting the receiver side of events.
    // This is little bit tricky.
    let (mut dtx, mut drx) = oneshot::channel::<()>();

    // When `drx` will dropped then `dtx` will be canceled.
    // We can track it to make sure when the user leaves chat.
    warp::spawn(poll_fn(move || dtx.poll_cancel()).map(move |_| {
        user_disconnected(my_id, &users2);
    }));

    // Convert messages into Server-Sent Events and return resulting stream.
    rx.map(|msg| match msg {
        Message::UserId(my_id) => (warp::sse::event("user"), warp::sse::data(my_id)).into_a(),
        Message::Reply(reply) => warp::sse::data(reply).into_b(),
    })
    .map_err(move |_| {
        // Keep `drx` alive until `rx` will be closed
        drx.close();
        unreachable!("unbounded rx never errors");
    })
}

fn user_message(my_id: usize, msg: String, users: &Users) {
    let new_msg = format!("<User#{}>: {}", my_id, msg);

    // New message from this user, send it to everyone else (except same uid)...
    //
    // We use `retain` instead of a for loop so that we can reap any user that
    // appears to have disconnected.
    for (&uid, tx) in users.lock().unwrap().iter() {
        if my_id != uid {
            match tx.unbounded_send(Message::Reply(new_msg.clone())) {
                Ok(()) => (),
                Err(_disconnected) => {
                    // The tx is disconnected, our `user_disconnected` code
                    // should be happening in another task, nothing more to
                    // do here.
                }
            }
        }
    }
}

fn user_disconnected(my_id: usize, users: &Users) {
    eprintln!("good bye user: {}", my_id);

    // Stream closed up, so remove from the user list
    users.lock().unwrap().remove(&my_id);
}

static INDEX_HTML: &str = r#"
<!DOCTYPE html>
<html>
    <head>
        <title>Warp Chat</title>
    </head>
    <body>
        <h1>warp chat</h1>
        <div id="chat">
            <p><em>Connecting...</em></p>
        </div>
        <input type="text" id="text" />
        <button type="button" id="send">Send</button>
        <script type="text/javascript">
        var uri = 'http://' + location.host + '/chat';
        var sse = new EventSource(uri);
        function message(data) {
            var line = document.createElement('p');
            line.innerText = data;
            chat.appendChild(line);
        }
        sse.onopen = function() {
            chat.innerHTML = "<p><em>Connected!</em></p>";
        }
        var user_id;
        sse.addEventListener("user", function(msg) {
            user_id = msg.data;
        });
        sse.onmessage = function(msg) {
            message(msg.data);
        };
        send.onclick = function() {
            var msg = text.value;
            var xhr = new XMLHttpRequest();
            xhr.open("POST", uri + '/' + user_id, true);
            xhr.send(msg);
            text.value = '';
            message('<You>: ' + msg);
        };
        </script>
    </body>
</html>
"#;
