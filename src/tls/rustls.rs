use std::io::{self, BufReader, Read};
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use futures_util::{ready, Future};

use hyper::server::accept::Accept;
use hyper::server::conn::{AddrIncoming, AddrStream};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio_rustls::rustls::{
    server::{AllowAnyAnonymousOrAuthenticatedClient, AllowAnyAuthenticatedClient, NoClientAuth},
    Certificate, PrivateKey, RootCertStore, ServerConfig,
};

use crate::tls::TlsClientAuth;
use crate::transport::Transport;

use super::{TlsConfigBuilder, TlsConfigError};

pub(crate) struct TlsAcceptor {
    config: Arc<ServerConfig>,
    incoming: AddrIncoming,
}

impl TlsAcceptor {
    pub(crate) fn new(config: ServerConfig, incoming: AddrIncoming) -> TlsAcceptor {
        TlsAcceptor {
            config: Arc::new(config),
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
            Some(Ok(sock)) => Poll::Ready(Some(Ok(TlsStream::new(sock, pin.config.clone())))),
            Some(Err(e)) => Poll::Ready(Some(Err(e))),
            None => Poll::Ready(None),
        }
    }
}

impl TlsConfigBuilder {
    pub(crate) fn build(mut self) -> Result<ServerConfig, TlsConfigError> {
        let mut cert_rdr = BufReader::new(self.cert);
        let cert = rustls_pemfile::certs(&mut cert_rdr)
            .map_err(|_e| TlsConfigError::CertParseError)?
            .into_iter()
            .map(Certificate)
            .collect();

        let key = {
            // convert it to Vec<u8> to allow reading it again if key is RSA
            let mut key_vec = Vec::new();
            self.key
                .read_to_end(&mut key_vec)
                .map_err(TlsConfigError::Io)?;

            if key_vec.is_empty() {
                return Err(TlsConfigError::EmptyKey);
            }

            let mut pkcs8 = rustls_pemfile::pkcs8_private_keys(&mut key_vec.as_slice())
                .map_err(|_e| TlsConfigError::Pkcs8ParseError)?;

            if !pkcs8.is_empty() {
                PrivateKey(pkcs8.remove(0))
            } else {
                let mut rsa = rustls_pemfile::rsa_private_keys(&mut key_vec.as_slice())
                    .map_err(|_e| TlsConfigError::RsaParseError)?;

                if !rsa.is_empty() {
                    PrivateKey(rsa.remove(0))
                } else {
                    return Err(TlsConfigError::EmptyKey);
                }
            }
        };

        fn read_trust_anchor(
            trust_anchor: Box<dyn Read + Send + Sync>,
        ) -> Result<RootCertStore, TlsConfigError> {
            let trust_anchors = {
                let mut reader = BufReader::new(trust_anchor);
                rustls_pemfile::certs(&mut reader).map_err(TlsConfigError::Io)?
            };

            let mut store = RootCertStore::empty();
            let (added, _skipped) = store.add_parsable_certificates(&trust_anchors);
            if added == 0 {
                return Err(TlsConfigError::CertParseError);
            }

            Ok(store)
        }

        let client_auth = match self.client_auth {
            TlsClientAuth::Off => NoClientAuth::boxed(),
            TlsClientAuth::Optional(trust_anchor) => AllowAnyAnonymousOrAuthenticatedClient::boxed(
                AllowAnyAnonymousOrAuthenticatedClient::new(read_trust_anchor(trust_anchor)?),
            ),
            TlsClientAuth::Required(trust_anchor) => AllowAnyAuthenticatedClient::boxed(
                AllowAnyAuthenticatedClient::new(read_trust_anchor(trust_anchor)?),
            ),
        };

        let mut config = ServerConfig::builder()
            .with_safe_defaults()
            .with_client_cert_verifier(client_auth.into())
            .with_single_cert_with_ocsp_and_sct(cert, key, self.ocsp_resp, Vec::new())
            .map_err(TlsConfigError::InvalidKey)?;
        config.alpn_protocols = vec!["h2".into(), "http/1.1".into()];
        Ok(config)
    }
}

impl Transport for TlsStream {
    fn remote_addr(&self) -> Option<SocketAddr> {
        Some(self.remote_addr)
    }
}

enum State {
    Handshaking(tokio_rustls::Accept<AddrStream>),
    Streaming(tokio_rustls::server::TlsStream<AddrStream>),
}

// tokio_rustls::server::TlsStream doesn't expose constructor methods,
// so we have to TlsAcceptor::accept and handshake to have access to it
// TlsStream implements AsyncRead/AsyncWrite handshaking tokio_rustls::Accept first
pub(crate) struct TlsStream {
    state: State,
    remote_addr: SocketAddr,
}

impl TlsStream {
    fn new(stream: AddrStream, config: Arc<ServerConfig>) -> TlsStream {
        let remote_addr = stream.remote_addr();
        let accept = tokio_rustls::TlsAcceptor::from(config).accept(stream);
        TlsStream {
            state: State::Handshaking(accept),
            remote_addr,
        }
    }
}

impl AsyncRead for TlsStream {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let pin = self.get_mut();
        match pin.state {
            State::Handshaking(ref mut accept) => match ready!(Pin::new(accept).poll(cx)) {
                Ok(mut stream) => {
                    let result = Pin::new(&mut stream).poll_read(cx, buf);
                    pin.state = State::Streaming(stream);
                    result
                }
                Err(err) => Poll::Ready(Err(err)),
            },
            State::Streaming(ref mut stream) => Pin::new(stream).poll_read(cx, buf),
        }
    }
}

impl AsyncWrite for TlsStream {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        let pin = self.get_mut();
        match pin.state {
            State::Handshaking(ref mut accept) => match ready!(Pin::new(accept).poll(cx)) {
                Ok(mut stream) => {
                    let result = Pin::new(&mut stream).poll_write(cx, buf);
                    pin.state = State::Streaming(stream);
                    result
                }
                Err(err) => Poll::Ready(Err(err)),
            },
            State::Streaming(ref mut stream) => Pin::new(stream).poll_write(cx, buf),
        }
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        match self.state {
            State::Handshaking(_) => Poll::Ready(Ok(())),
            State::Streaming(ref mut stream) => Pin::new(stream).poll_flush(cx),
        }
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        match self.state {
            State::Handshaking(_) => Poll::Ready(Ok(())),
            State::Streaming(ref mut stream) => Pin::new(stream).poll_shutdown(cx),
        }
    }
}
