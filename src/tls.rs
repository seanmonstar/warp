//! This module provides helpers for enabling tls  
use std::fs::File;
use std::io::{self, BufReader, Read, Write};
use std::net::SocketAddr;
use std::path::Path;

use futures::Poll;
use rustls::{self, ServerConfig, ServerSession, Session, Stream};
use tokio_io::{AsyncRead, AsyncWrite};

use transport::Transport;

/// Create a rustls ServerConfig from filesystem paths. This targets the most
/// commonly used case of single certificate in the chain. 
/// 
/// For anything else, use the `config` fn.
pub fn config_from_path(cert: impl AsRef<Path>, key: impl AsRef<Path>) -> ServerConfig {
    let cert_chain = {
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
    config(cert_chain, key)
}

/// Create a rustls ServerConfig from raw DER data. This targets the most
/// commonly used case of single certificate in the chain. 
/// 
/// For anything else, use the `config` fn.
pub fn config_from_der_data(cert: Vec<u8>, key: Vec<u8>) -> ServerConfig {
    let cert = rustls::Certificate(cert);
    let key = rustls::PrivateKey(key);
    config(vec![cert], key)
}

/// Create a rustls ServerConfig from cert chain and private key 
pub fn config(cert_chain: Vec<rustls::Certificate>, key: rustls::PrivateKey) -> ServerConfig {
    let mut tls = ServerConfig::new(rustls::NoClientAuth::new());
    tls.set_single_cert(cert_chain, key)
        .unwrap_or_else(|e| panic!("tls failed: {}", e));
    tls.set_protocols(&["h2".into(), "http/1.1".into()]);
    tls
}

pub(crate) fn configure(cert: &Path, key: &Path) -> ServerConfig {
    config_from_path(cert, key)
}

/// A TlsStream that lazily does ths TLS handshake.
#[derive(Debug)]
pub(crate) struct TlsStream<T> {
    io: T,
    is_shutdown: bool,
    session: ServerSession,
}

impl<T> TlsStream<T> {
    pub(crate) fn new(io: T, session: ServerSession) -> Self {
        TlsStream {
            io,
            is_shutdown: false,
            session,
        }
    }
}

impl<T: Read + Write> Read for TlsStream<T> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        Stream::new(&mut self.session, &mut self.io).read(buf)
    }
}

impl<T: Read + Write> Write for TlsStream<T> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        Stream::new(&mut self.session, &mut self.io).write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        Stream::new(&mut self.session, &mut self.io).flush()?;
        self.io.flush()
    }
}

impl<T: AsyncRead + AsyncWrite> AsyncRead for TlsStream<T> {}

impl<T: AsyncRead + AsyncWrite> AsyncWrite for TlsStream<T> {
    fn shutdown(&mut self) -> Poll<(), io::Error> {
        if self.session.is_handshaking() {
            return Ok(().into());
        }

        if !self.is_shutdown {
            self.session.send_close_notify();
            self.is_shutdown = true;
        }

        try_nb!(self.flush());
        self.io.shutdown()
    }
}

impl<T: Transport> Transport for TlsStream<T> {
    fn remote_addr(&self) -> Option<SocketAddr> {
        self.io.remote_addr()
    }
}
