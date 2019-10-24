use std::fs::File;
use std::io::{self, BufReader, Read, Write};
use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;
use std::pin::Pin;
use std::ptr::null_mut;
use std::task::{Poll, Context};

use futures::ready;
use rustls::{self, ServerConfig, ServerSession, Session, Stream};
use hyper::server::accept::Accept;
use hyper::server::conn::{AddrIncoming, AddrStream};
use tokio::io::{AsyncRead, AsyncWrite};

use crate::transport::Transport;

pub(crate) fn configure(cert: &Path, key: &Path) -> ServerConfig {
    let cert = {
        let file = File::open(cert).unwrap_or_else(|e| panic!("tls cert file error: {}", e));
        let mut rdr = BufReader::new(file);
        rustls::internal::pemfile::certs(&mut rdr)
            .unwrap_or_else(|()| panic!("tls cert parse error"))
    };

    let key = {
        let mut pkcs8 = {
            let file = File::open(&key).unwrap_or_else(|e| panic!("tls key file error: {}", e));
            let mut rdr = BufReader::new(file);
            rustls::internal::pemfile::pkcs8_private_keys(&mut rdr)
                .unwrap_or_else(|()| panic!("tls key pkcs8 error"))
        };

        if !pkcs8.is_empty() {
            pkcs8.remove(0)
        } else {
            let file = File::open(key).unwrap_or_else(|e| panic!("tls key file error: {}", e));
            let mut rdr = BufReader::new(file);
            let mut rsa = rustls::internal::pemfile::rsa_private_keys(&mut rdr)
                .unwrap_or_else(|()| panic!("tls key rsa error"));

            if !rsa.is_empty() {
                rsa.remove(0)
            } else {
                panic!("tls key path contains no private key");
            }
        }
    };

    let mut tls = ServerConfig::new(rustls::NoClientAuth::new());
    tls.set_single_cert(cert, key)
        .unwrap_or_else(|e| panic!("tls failed: {}", e));
    tls.set_protocols(&["h2".into(), "http/1.1".into()]);
    tls
}

/// a wrapper arround T to allow for rustls Stream read/write translations to async read and write
#[derive(Debug)]
struct AllowStd<T> {
    inner: T,
    context: *mut (),
}

// *mut () context is neither Send nor Sync
unsafe impl<T: Send> Send for AllowStd<T> {}
unsafe impl<T: Sync> Sync for AllowStd<T> {}

struct Guard<'a, T>(&'a mut TlsStream<T>)
where
    AllowStd<T>: Read + Write;

impl<T> Drop for Guard<'_, T>
where
    AllowStd<T>: Read + Write,
{
    fn drop(&mut self) {
        (self.0).io.context = null_mut();
    }
}

impl<T> AllowStd<T>
where
    T: Unpin,
{
    fn with_context<F, R>(&mut self, f: F) -> R
    where
        F: FnOnce(&mut Context<'_>, Pin<&mut T>) -> R,
    {
        unsafe {
            assert!(!self.context.is_null());
            let waker = &mut *(self.context as *mut _);
            f(waker, Pin::new(&mut self.inner))
        }
    }
}

impl<T> Read for AllowStd<T>
where
    T: AsyncRead + Unpin,
{
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self.with_context(|ctx, stream| stream.poll_read(ctx, buf)) {
            Poll::Ready(r) => r,
            Poll::Pending => Err(io::Error::from(io::ErrorKind::WouldBlock)),
        }
    }
}

impl<T> Write for AllowStd<T>
where
    T: AsyncWrite + Unpin,
{
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self.with_context(|ctx, stream| stream.poll_write(ctx, buf)) {
            Poll::Ready(r) => r,
            Poll::Pending => Err(io::Error::from(io::ErrorKind::WouldBlock)),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match self.with_context(|ctx, stream| stream.poll_flush(ctx)) {
            Poll::Ready(r) => r,
            Poll::Pending => Err(io::Error::from(io::ErrorKind::WouldBlock)),
        }
    }
}

fn cvt<T>(r: io::Result<T>) -> Poll<io::Result<T>> {
    match r {
        Ok(v) => Poll::Ready(Ok(v)),
        Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => Poll::Pending,
        Err(e) => Poll::Ready(Err(e)),
    }
}

/// A TlsStream that lazily does ths TLS handshake.
#[derive(Debug)]
pub(crate) struct TlsStream<T> {
    io: AllowStd<T>,
    is_shutdown: bool,
    session: ServerSession,
}

impl<T> TlsStream<T> {
    pub(crate) fn new(io: T, session: ServerSession) -> Self {
        TlsStream {
            io: AllowStd{ inner: io, context: null_mut() },
            is_shutdown: false,
            session,
        }
    }

    fn with_context<F, R>(&mut self, ctx: &mut Context<'_>, f: F) -> R
    where
        F: FnOnce(&mut AllowStd<T>, &mut ServerSession) -> R,
        AllowStd<T>: Read + Write,
    {
        self.io.context = ctx as *mut _ as *mut ();
        let g = Guard(self);
        f(&mut (g.0).io, &mut (g.0).session)
    }
}

impl<T: AsyncRead + AsyncWrite + Unpin> AsyncRead for TlsStream<T> {

    fn poll_read(mut self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut [u8]) -> Poll<io::Result<usize>> {
        self.with_context(cx, |io, session| {
            cvt(Stream::new(session, io).read(buf))
        })
    }

}

impl<T: AsyncRead + AsyncWrite + Unpin> AsyncWrite for TlsStream<T> {

    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        self.with_context(cx, |io, session| {
            cvt(Stream::new(session, io).write(buf))
        })
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        self.with_context(cx, |io, session| {
            if let Err(e) = ready!(cvt(Stream::new(session, io).flush())) {
                return Poll::Ready(Err(e));
            }
            cvt(io.flush())
        })
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let mut pin = self.get_mut();
        if pin.session.is_handshaking() {
            return Poll::Ready(Ok(()));
        }

        if !pin.is_shutdown {
            pin.session.send_close_notify();
            pin.is_shutdown = true;
        }

        if let Err(e) = ready!(Pin::new(&mut pin).poll_flush(cx)) {
            return Poll::Ready(Err(e));
        }

        Pin::new(&mut pin.io.inner).poll_shutdown(cx)
    }
}

impl<T: Transport + Unpin> Transport for TlsStream<T> {
    fn remote_addr(&self) -> Option<SocketAddr> {
        self.io.inner.remote_addr()
    }
}

pub(crate) struct TlsAcceptor {
    config: Arc<ServerConfig>,
    incoming: AddrIncoming,
}

impl TlsAcceptor {
    pub(crate) fn new(config: ServerConfig, incoming: AddrIncoming) -> TlsAcceptor {
        TlsAcceptor{ config: Arc::new(config), incoming }
    }
}

impl Accept for TlsAcceptor {
    type Conn = TlsStream<AddrStream>;
    type Error = io::Error;

    fn poll_accept(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Result<Self::Conn, Self::Error>>> {
        let pin = self.get_mut();
        match ready!(Pin::new(&mut pin.incoming).poll_accept(cx)) {
            Some(Ok(sock)) => {
                let session = ServerSession::new(&pin.config.clone());
                // let tls = Arc::new($this.config);
                return Poll::Ready(Some(Ok(TlsStream::new(sock, session))));
            },
            Some(Err(e)) => Poll::Ready(Some(Err(e))),
            None => Poll::Ready(None)
        }
    }
}