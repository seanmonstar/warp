use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::{
    io::{self, Read},
    task,
};

use futures_util::ready;
use hyper::server::accept::Accept;
use hyper::server::conn::{AddrIncoming, AddrStream};
use openssl::pkey::PKey;
use openssl::ssl::{Ssl, SslAcceptor, SslMethod, SslVerifyMode};
use openssl::x509::store::{X509Store, X509StoreBuilder};
use openssl::x509::X509;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_openssl::SslStream;

use crate::transport::Transport;

use super::{TlsClientAuth, TlsConfigBuilder, TlsConfigError};

impl TlsConfigBuilder {
    pub(crate) fn build(mut self) -> Result<SslConfig, TlsConfigError> {
        let mut key_vec = Vec::new();
        self.key
            .read_to_end(&mut key_vec)
            .map_err(TlsConfigError::Io)?;

        if key_vec.is_empty() {
            return Err(TlsConfigError::EmptyKey);
        }

        let private_key =
            PKey::private_key_from_pem(&key_vec).map_err(TlsConfigError::OpensslError)?;

        let mut cert_vec = Vec::new();
        self.cert
            .read_to_end(&mut cert_vec)
            .map_err(TlsConfigError::Io)?;

        let mut cert_chain = X509::stack_from_pem(&cert_vec)
            .map_err(TlsConfigError::OpensslError)?
            .into_iter();
        let cert = cert_chain.next().ok_or(TlsConfigError::EmptyCert)?;
        let chain: Vec<_> = cert_chain.collect();
        let mut acceptor = SslAcceptor::mozilla_intermediate(SslMethod::tls())
            .map_err(TlsConfigError::OpensslError)?;
        acceptor
            .set_private_key(&private_key)
            .map_err(TlsConfigError::OpensslError)?;
        acceptor
            .set_certificate(&cert)
            .map_err(TlsConfigError::OpensslError)?;

        for cert in chain.iter() {
            acceptor
                .add_extra_chain_cert(cert.to_owned())
                .map_err(TlsConfigError::OpensslError)?;
        }

        acceptor
            .set_alpn_protos(b"\x02h2\x08http/1.1")
            .map_err(TlsConfigError::OpensslError)?;

        fn read_trust_anchor(
            mut trust_anchor: Box<dyn Read + Send + Sync>,
        ) -> Result<X509Store, TlsConfigError> {
            let mut cert_vec = Vec::new();
            trust_anchor
                .read_to_end(&mut cert_vec)
                .map_err(TlsConfigError::Io)?;

            let certs = X509::stack_from_pem(&cert_vec).map_err(TlsConfigError::OpensslError)?;
            let mut store = X509StoreBuilder::new().map_err(TlsConfigError::OpensslError)?;

            for cert in certs.into_iter() {
                store.add_cert(cert).map_err(TlsConfigError::OpensslError)?;
            }

            Ok(store.build())
        }

        match self.client_auth {
            TlsClientAuth::Off => acceptor.set_verify(SslVerifyMode::NONE),
            TlsClientAuth::Optional(trust_anchor) => {
                let store = read_trust_anchor(trust_anchor)?;
                acceptor.set_verify(SslVerifyMode::PEER);
                acceptor
                    .set_verify_cert_store(store)
                    .map_err(TlsConfigError::OpensslError)?;
            }
            TlsClientAuth::Required(trust_anchor) => {
                let store = read_trust_anchor(trust_anchor)?;
                acceptor.set_verify(SslVerifyMode::PEER | SslVerifyMode::FAIL_IF_NO_PEER_CERT);
                acceptor
                    .set_verify_cert_store(store)
                    .map_err(TlsConfigError::OpensslError)?;
            }
        };

        Ok(SslConfig {
            acceptor: acceptor.build(),
            ocsp_response: self.ocsp_resp,
        })
    }
}

pub(crate) struct SslConfig {
    acceptor: SslAcceptor,
    ocsp_response: Vec<u8>,
}

pub(crate) struct TlsAcceptor {
    ssl_config: Arc<SslConfig>,
    incoming: AddrIncoming,
}

impl TlsAcceptor {
    pub(crate) fn new(ssl_config: SslConfig, incoming: AddrIncoming) -> TlsAcceptor {
        TlsAcceptor {
            ssl_config: Arc::new(ssl_config),
            incoming,
        }
    }
}

impl Accept for TlsAcceptor {
    type Conn = TlsStream;
    type Error = io::Error;

    fn poll_accept(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Self::Conn, Self::Error>>> {
        let pin = self.get_mut();
        match ready!(Pin::new(&mut pin.incoming).poll_accept(cx)) {
            Some(Ok(sock)) => Poll::Ready(Some(Ok(TlsStream::new(sock, pin.ssl_config.clone())?))),
            Some(Err(e)) => Poll::Ready(Some(Err(e))),
            None => Poll::Ready(None),
        }
    }
}
enum State {
    Handshaking,
    Streaming,
}

pub(crate) struct TlsStream {
    state: State,
    stream: SslStream<AddrStream>,
    remote_addr: SocketAddr,
}

impl Transport for TlsStream {
    fn remote_addr(&self) -> Option<SocketAddr> {
        Some(self.remote_addr)
    }
}

impl TlsStream {
    fn new(stream: AddrStream, ssl_config: Arc<SslConfig>) -> Result<TlsStream, io::Error> {
        let remote_addr = stream.remote_addr();
        let mut ssl = Ssl::new(ssl_config.acceptor.context()).map_err(io::Error::from)?;

        if ssl_config.ocsp_response.len() > 0 {
            ssl.set_ocsp_status(&ssl_config.ocsp_response)
                .map_err(io::Error::from)?;
        }

        let stream = SslStream::new(ssl, stream).map_err(io::Error::from)?;
        Ok(TlsStream {
            state: State::Handshaking,
            stream,
            remote_addr,
        })
    }
}

impl AsyncRead for TlsStream {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        match self.state {
            State::Handshaking => match ready!(Pin::new(&mut self.stream).poll_accept(cx)) {
                Ok(_) => {
                    self.state = State::Streaming;
                    let result = Pin::new(&mut self.stream).poll_read(cx, buf);
                    result
                }
                Err(err) => Poll::Ready(Err(err
                    .into_io_error()
                    .unwrap_or_else(|e| io::Error::new(io::ErrorKind::Other, e)))),
            },
            State::Streaming => Pin::new(&mut self.stream).poll_read(cx, buf),
        }
    }
}

impl AsyncWrite for TlsStream {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> task::Poll<Result<usize, io::Error>> {
        match self.state {
            State::Handshaking => match ready!(Pin::new(&mut self.stream).poll_accept(cx)) {
                Ok(_) => {
                    self.state = State::Streaming;
                    let result = Pin::new(&mut self.stream).poll_write(cx, buf);
                    result
                }
                Err(err) => Poll::Ready(Err(err
                    .into_io_error()
                    .unwrap_or_else(|e| io::Error::new(io::ErrorKind::Other, e)))),
            },
            State::Streaming => Pin::new(&mut self.stream).poll_write(cx, buf),
        }
    }

    fn poll_flush(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> task::Poll<Result<(), io::Error>> {
        match self.state {
            State::Handshaking => Poll::Ready(Ok(())),
            State::Streaming => Pin::new(&mut self.stream).poll_flush(cx),
        }
    }

    fn poll_shutdown(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> task::Poll<Result<(), io::Error>> {
        match self.state {
            State::Handshaking => Poll::Ready(Ok(())),
            State::Streaming => Pin::new(&mut self.stream).poll_shutdown(cx),
        }
    }
}
