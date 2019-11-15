//! TlsConfigBuilder
/// A builder to configure warp using Tls
///
/// # Example
///
/// ```no_run
/// #[tokio::main]
/// async fn main() {
///    use warp::Filter;
///
///    // Match any request and return hello world!
///    let routes = warp::any().map(|| "Hello, World!");
///
///    let mut tls_config = warp::tls::TlsConfigBuilder::new();
///    tls_config
///        .set_cert_path("examples/tls/cert.pem").unwrap()
///        .set_key_path("examples/tls/key.rsa").unwrap();
///
///    warp::serve(routes)
///        .tls(&mut tls_config)
///        .run(([127, 0, 0, 1], 3030)).await;
/// }
/// ```

use std::fs::File;
use std::io::{self, BufReader, Cursor, Read, Write};
use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;
use std::pin::Pin;
use std::ptr::null_mut;
use std::task::{Poll, Context};

use futures::ready;
use rustls::{self, ServerConfig, ServerSession, Session, Stream, TLSError};
use hyper::server::accept::Accept;
use hyper::server::conn::{AddrIncoming, AddrStream};
use tokio::io::{AsyncRead, AsyncWrite};

use crate::transport::Transport;

/// Represents errors that can occur building the TlsConfig
#[derive(Debug)]
pub enum TlsConfigError {
    /// An Error from an invalid file
    IoError(std::io::Error),
    /// An Error parsing the Certificate
    CertParseError,
    /// An Error parsing a Pkcs8 key
    Pkcs8ParseError,
    /// An Error parsing a Rsa key
    RsaParseError,
    /// An error from an empty key
    EmptyKey,
    /// An error from an invalid key
    InvalidKey(TLSError)
}

impl std::fmt::Display for TlsConfigError{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TlsConfigError::IoError(err) => write!(f, "file error, {}", err),
            TlsConfigError::CertParseError => write!(f, "certificate parse error"),
            TlsConfigError::Pkcs8ParseError => write!(f, "pkcs8 parse error"),
            TlsConfigError::RsaParseError => write!(f, "rsa parse error"),
            TlsConfigError::EmptyKey => write!(f, "key contains no private key"),
            TlsConfigError::InvalidKey(err) => write!(f, "key contains an invalid key, {}", err),
        }
    }
}

/// Builder to set the configuration for the Tls server.
pub struct TlsConfigBuilder {
    cert: BufReader<Box<dyn Read>>,
    key: BufReader<Box<dyn Read>>,
}

impl std::fmt::Debug for TlsConfigBuilder {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        f.debug_struct("TlsConfigBuilder")
            .field("cert", &String::from_utf8_lossy(&self.cert.buffer()))
            .field("key", &String::from_utf8_lossy(&self.key.buffer()))
            .finish()
    }
}

impl TlsConfigBuilder {
    /// Create a new TlsConfigBuilder
    pub fn new() -> TlsConfigBuilder {
        TlsConfigBuilder{ key: BufReader::new(Box::new(io::empty())), cert: BufReader::new(Box::new(io::empty())) }
    }

    /// sets the Tls key via File Path, returns `TlsConfigError::IoError` if the file cannot be open
    pub fn set_key_path<'a>(&'a mut self, path: impl AsRef<Path>) -> Result<&'a mut Self, TlsConfigError> {
        let key = File::open(&path).map_err(|e| TlsConfigError::IoError(e))?;
        self.key = BufReader::new(Box::new(key));
        Ok(self)
    }

    /// sets the Tls key via bytes slice
    pub fn set_key<'a>(&'a mut self, key: &[u8]) -> &'a Self {
        let cursor = Cursor::new(Vec::from(key));
        self.key = BufReader::new(Box::new(cursor));
        self
    }

    /// sets the Tls certificate via File Path, returns `TlsConfigError::IoError` if the file cannot be open
    pub fn set_cert_path<'a>(&'a mut self, path: impl AsRef<Path>) -> Result<&'a mut Self, TlsConfigError> {
        let cert = File::open(&path).map_err(|e| TlsConfigError::IoError(e))?;
        self.cert = BufReader::new(Box::new(cert));
        Ok(self)
    }

    /// sets the Tls certificate via bytes slice
    pub fn set_cert<'a>(&'a mut self, cert: &[u8]) -> &'a Self {
        let cursor = Cursor::new(Vec::from(cert));
        self.cert = BufReader::new(Box::new(cursor));
        self
    }

    pub(crate) fn build<'a>(&'a mut self) -> Result<ServerConfig, TlsConfigError> {
        let cert = rustls::internal::pemfile::certs(&mut self.cert)
            .map_err(|()| TlsConfigError::CertParseError)?;

        let key = {
            // convert it to Vec<u8> to allow reading it again if key is RSA

            let mut pkcs8_buf = BufReader::new(self.key.buffer());

            let mut pkcs8 = rustls::internal::pemfile::pkcs8_private_keys(&mut pkcs8_buf)
                .map_err(|()| TlsConfigError::Pkcs8ParseError)?;

            if !pkcs8.is_empty() {
                pkcs8.remove(0)
            } else {
                let mut rsa = rustls::internal::pemfile::rsa_private_keys(&mut self.key)
                    .map_err(|()| TlsConfigError::RsaParseError)?;

                    if !rsa.is_empty() {
                        rsa.remove(0)
                    } else {
                        return Err(TlsConfigError::EmptyKey);
                    }
            }
        };

        let mut config = ServerConfig::new(rustls::NoClientAuth::new());
        config.set_single_cert(cert, key)
            .map_err(|err| TlsConfigError::InvalidKey(err))?;
        config.set_protocols(&["h2".into(), "http/1.1".into()]);
        Ok(config)
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_cert_key() {
        let mut builder = TlsConfigBuilder::new();
        assert!(builder.set_key_path("examples/tls/key.rsa").is_ok());
        assert!(builder.set_cert_path("examples/tls/cert.pem").is_ok());
        assert!(builder.build().is_ok())
    }

    #[test]
    fn bytes_cert_key() {
        let key = include_str!("../examples/tls/key.rsa");
        let cert = include_str!("../examples/tls/cert.pem");

        let mut builder = TlsConfigBuilder::new();
        builder.set_key(key.as_bytes());
        builder.set_cert(cert.as_bytes());
        assert!(builder.build().is_ok())
    }
}