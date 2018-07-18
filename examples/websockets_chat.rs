extern crate pretty_env_logger;
extern crate warp;

use std::collections::HashMap;
use std::sync::{Arc, Mutex, atomic::{AtomicUsize, Ordering}};

use warp::{Filter, Future, Sink, Stream};
use warp::ws::Message;

static NEXT_USER_ID: AtomicUsize = AtomicUsize::new(1);

fn main() {
    pretty_env_logger::init();

    // Keep track of all connected users, key is usize, value
    // is a websocket sender.
    let users = Arc::new(Mutex::new(HashMap::new()));

    // The `ws()` filter will do the full Websocket handshake,
    // and call our function if the handshake succeeds.
    let ws = warp::ws(move |websocket| {
        // Use a counter to assign a new unique ID for this user.
        let my_id = NEXT_USER_ID.fetch_add(1, Ordering::Relaxed);

        eprintln!("new chat user: {}", my_id);

        // Split the socket into a sender and receive of messages.
        let (tx, rx) = websocket.split();

        // Save the sender in our list of connected users.
        users
            .lock()
            .unwrap()
            .insert(my_id, tx);

        let users = users.clone();
        let users2 = users.clone();
        rx
            .for_each(move |msg| {
                // New message from this user, send it to
                // everyone else (except same uid)...
                let msg = if let Ok(s) = msg.to_str() {
                    s
                } else{
                    // Skip any non-Text messages...
                    return Ok(());
                };
                let new_msg = format!("<User#{}>: {}", my_id, msg);
                for (&uid, tx) in users.lock().unwrap().iter_mut() {
                    if my_id != uid {
                        let _ = tx.start_send(Message::text(new_msg.clone()));
                    }
                }
                Ok(())
            })
            .then(move |res| {
                eprintln!("good bye user: {}", my_id);

                // Stream closed up, so remove from the user list
                users2
                    .lock()
                    .unwrap()
                    .remove(&my_id);

                res
            })
            .map_err(move |e| {
                eprintln!("websocket error(uid={}): {:?}", my_id, e);
            })
    });

    let chat = warp::path("chat").and(ws);
    let index = warp::reply::with::header("content-type", "text/html; charset=utf-8")
        .decorate(warp::path::index().map(|| INDEX_HTML));

    let routes = index.or(chat);

    warp::serve(routes)
        .run(([127, 0, 0, 1], 3030));
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
