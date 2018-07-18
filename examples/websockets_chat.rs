extern crate pretty_env_logger;
extern crate warp;

use std::collections::HashMap;
use std::sync::{Arc, Mutex, atomic::{AtomicUsize, Ordering, ATOMIC_USIZE_INIT}};

use warp::{Filter, Future, Sink, Stream};
use warp::ws::Message;

static NEXT_USER_ID: AtomicUsize = ATOMIC_USIZE_INIT;

fn main() {
    pretty_env_logger::init();

    let users = Arc::new(Mutex::new(HashMap::new()));

    // The `ws()` filter will do the full Websocket handshake,
    // and call our function if the handshake succeeds.
    let ws = warp::ws(move |websocket| {
        let my_id = NEXT_USER_ID.fetch_add(1, Ordering::Relaxed);

        eprintln!("new chat user: {}", my_id);

        let (tx, rx) = websocket.split();

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
                let mut new_msg = format!("From #{}", my_id).into_bytes();
                new_msg.extend(msg.as_bytes());
                for (&uid, tx) in users.lock().unwrap().iter_mut() {
                    if my_id != uid {
                        let _ = tx.start_send(Message::binary(new_msg.clone()));
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
    let index = warp::path::index().map(|| INDEX_HTML);

    let routes = index.or(chat);

    warp::serve(routes)
        .run(([127, 0, 0, 1], 3030));
}

static INDEX_HTML: &str = r#"
<html>
    <head>
        <title>Chat</title>
    </head>
    <body>
        <h1>warp chat</h1>
        <div id="chat"></div>
        <input type="text" id="text" />
        <button type="button" id="send">Send</button>
        <script type="text/javascript">
        var uri = 'ws://' + location.host + '/chat';
        var ws = new WebSocket(uri);

        function message(data) {
            var line = document.createElement('p');
            line.innerText = msg.data;
            chat.appendChild(line);
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
