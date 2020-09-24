#![deny(warnings)]

use std::io::Error;
use std::net::SocketAddr;
use std::pin::Pin;
use std::task::{Context, Poll};

use warp::Filter;
use warp::Transport;

use futures::Stream;
use tokio::io::AsyncRead;
use tokio::net::{TcpListener, TcpStream};
use tokio::prelude::AsyncWrite;

pub struct MyS {
    stream: TcpStream,
}

impl Transport for MyS {
    fn remote_addr(&self) -> Option<SocketAddr> {
        match self.stream.peer_addr() {
            Ok(o) => Some(o),
            Err(_) => None,
        }
    }
}

impl AsyncRead for MyS {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<Result<usize, std::io::Error>> {
        let ptr = self.get_mut();
        Pin::new(&mut ptr.stream).poll_read(cx, buf)
    }
}

impl AsyncWrite for MyS {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, Error>> {
        let ptr = self.get_mut();
        Pin::new(&mut ptr.stream).poll_write(cx, buf)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        let ptr = self.get_mut();
        Pin::new(&mut ptr.stream).poll_flush(cx)
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        let ptr = self.get_mut();
        Pin::new(&mut ptr.stream).poll_shutdown(cx)
    }
}

pub struct MyIncoming {
    listen: TcpListener,
}

impl Stream for MyIncoming {
    type Item = Result<MyS, std::io::Error>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let ptr = self.get_mut();
        match Pin::new(&mut ptr.listen).poll_next(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(o) => match o {
                Some(item) => match item {
                    Ok(o) => Poll::Ready(Some(Ok(MyS { stream: o }))),
                    Err(e) => Poll::Ready(Some(Err(e))),
                },
                None => Poll::Ready(None),
            },
        }
    }
}

#[tokio::main]
async fn main() {
    // Match any request and return hello world!
    let routes = warp::addr::remote().map(|remote: Option<SocketAddr>| format!("{:?}", remote));

    let srv = warp::serve(routes);

    let s = "127.0.0.1:3000";
    let addr = s.parse::<SocketAddr>().unwrap();
    let listen = TcpListener::bind(&addr).await.unwrap();

    let my = MyIncoming { listen };

    srv.run_incoming(my).await;
}
