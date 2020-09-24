use std::io;
use std::net::SocketAddr;
use std::pin::Pin;
use std::task::{Context, Poll};

use hyper::server::conn::AddrStream;
use tokio::io::{AsyncRead, AsyncWrite};

/// impl Transport for your custom `Incoming`
/// to ensure `ws::addr::remote` return meaningful value
pub trait Transport: AsyncRead + AsyncWrite {
    /// get underline stream's remote addr
    fn remote_addr(&self) -> Option<SocketAddr>;
}

impl Transport for AddrStream {
    fn remote_addr(&self) -> Option<SocketAddr> {
        Some(self.remote_addr())
    }
}

/// LiftIo to support `Transport`
/// default always return None for `ws::addr::remote`
pub struct LiftIo<T>(pub T);

impl<T: AsyncRead + Unpin> AsyncRead for LiftIo<T> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut self.get_mut().0).poll_read(cx, buf)
    }
}

impl<T: AsyncWrite + Unpin> AsyncWrite for LiftIo<T> {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut self.get_mut().0).poll_write(cx, buf)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
        Pin::new(&mut self.get_mut().0).poll_flush(cx)
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
        Pin::new(&mut self.get_mut().0).poll_shutdown(cx)
    }
}

impl<T: AsyncRead + AsyncWrite + Unpin> Transport for LiftIo<T> {
    fn remote_addr(&self) -> Option<SocketAddr> {
        None
    }
}
