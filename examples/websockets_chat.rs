#![deny(warnings)]
extern crate futures;
extern crate pretty_env_logger;
extern crate warp;

use std::collections::HashMap;
use std::sync::{Arc, Mutex, atomic::{AtomicUsize, Ordering}};

use futures::{Future, Sink, Stream};
use futures::stream::SplitSink;
use warp::Filter;
use warp::ws::{Message, WebSocket};

/// Our global unique user id counter.
static NEXT_USER_ID: AtomicUsize = AtomicUsize::new(1);

/// Our state of currently connected users.
///
/// - Key is their id
/// - Value is a sender of `warp::ws::Message`s
type Users = Arc<Mutex<HashMap<usize, SplitSink<WebSocket>>>>;

fn main() {
    pretty_env_logger::init();

    // Keep track of all connected users, key is usize, value
    // is a websocket sender.
    let users = Arc::new(Mutex::new(HashMap::new()));

    // The `ws()` filter will do the full Websocket handshake,
    // and call our function if the handshake succeeds.
    let ws = warp::ws(move |socket| user_connected(socket, users.clone()));

    // GET /chat -> websocket upgrade
    let chat = warp::path("chat").and(ws);

    // GET / -> index html
    let index = warp::path::index()
        .map(|| {
            warp::http::Response::builder()
                .header("content-type", "text/html; charset=utf-8")
                .body(INDEX_HTML)
        });

    let routes = index.or(chat);

    warp::serve(routes)
        .run(([127, 0, 0, 1], 3030));
}

fn user_connected(ws: WebSocket, users: Users) -> impl Future<Item = (), Error = ()> {
    // Use a counter to assign a new unique ID for this user.
    let my_id = NEXT_USER_ID.fetch_add(1, Ordering::Relaxed);

    eprintln!("new chat user: {}", my_id);

    // Split the socket into a sender and receive of messages.
    let (tx, user_ws_rx) = ws.split();

    // Save the sender in our list of connected users.
    users
        .lock()
        .unwrap()
        .insert(my_id, tx);

    // Return a `Future` that is basically a state machine managing
    // this specific user's connection.

    // Make an extra clone to give to our disconnection handler...
    let users2 = users.clone();

    user_ws_rx
        // Every time the user sends a message, broadcast it to
        // all other users...
        .for_each(move |msg| {
            user_message(my_id, msg, &users);
            Ok(())
        })
        // for_each will keep processing as long as the user stays
        // connected. Once they disconnect, then...
        .then(move |result| {
            user_disconnected(my_id, &users2);
            result
        })
        // If at any time, there was a websocket error, log here...
        .map_err(move |e| {
            eprintln!("websocket error(uid={}): {}", my_id, e);
        })
}

fn user_message(my_id: usize, msg: Message, users: &Users) {
    // Skip any non-Text messages...
    let msg = if let Ok(s) = msg.to_str() {
        s
    } else{
        return;
    };

    let new_msg = format!("<User#{}>: {}", my_id, msg);

    // New message from this user, send it to
    // everyone else (except same uid)...
    for (&uid, tx) in users.lock().unwrap().iter_mut() {
        if my_id != uid {
            let _ = tx.start_send(Message::text(new_msg.clone()));
        }
    }
}

fn user_disconnected(my_id: usize, users: &Users) {
    eprintln!("good bye user: {}", my_id);

    // Stream closed up, so remove from the user list
    users
        .lock()
        .unwrap()
        .remove(&my_id);
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
        var uri = 'ws://' + location.host + '/chat';
        var ws = new WebSocket(uri);

        function message(data) {
            var line = document.createElement('p');
            line.innerText = data;
            chat.appendChild(line);
        }

        ws.onopen = function() {
            chat.innerHTML = "<p><em>Connected!</em></p>";
        }

        ws.onmessage = function(msg) {
            message(msg.data);
        };

        send.onclick = function() {
            var msg = text.value;
            ws.send(msg);
            text.value = '';

            message('<You>: ' + msg);
        };
        </script>
    </body>
</html>
"#;
