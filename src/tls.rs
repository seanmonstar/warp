use std::fmt;
use std::fs::File;
use std::io::{self, Cursor, Read};
use std::path::{Path, PathBuf};

#[cfg(feature = "tls")]
mod rustls;
#[cfg(feature = "tls")]
pub(crate) use rustls::TlsAcceptor;

#[cfg(feature = "tls-openssl")]
mod openssl;
#[cfg(feature = "tls-openssl")]
pub(crate) use self::openssl::TlsAcceptor;

/// Represents errors that can occur building the TlsConfig
#[derive(Debug)]
pub(crate) enum TlsConfigError {
    Io(io::Error),
    /// An Error parsing the Certificate
    #[cfg(feature = "tls")]
    CertParseError,
    /// An Error parsing a Pkcs8 key
    #[cfg(feature = "tls")]
    Pkcs8ParseError,
    /// An Error parsing a Rsa key
    #[cfg(feature = "tls")]
    RsaParseError,
    /// An error from an empty key
    EmptyKey,
    #[cfg(feature = "tls")]
    /// An error from an invalid key
    InvalidKey(tokio_rustls::rustls::Error),
    /// No public certificate was found
    #[cfg(feature = "tls-openssl")]
    EmptyCert,
    /// An error from openssl
    #[cfg(feature = "tls-openssl")]
    OpensslError(::openssl::error::ErrorStack),
}

impl fmt::Display for TlsConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TlsConfigError::Io(err) => err.fmt(f),
            #[cfg(feature = "tls")]
            TlsConfigError::CertParseError => write!(f, "certificate parse error"),
            #[cfg(feature = "tls")]
            TlsConfigError::Pkcs8ParseError => write!(f, "pkcs8 parse error"),
            #[cfg(feature = "tls")]
            TlsConfigError::RsaParseError => write!(f, "rsa parse error"),
            TlsConfigError::EmptyKey => write!(f, "key contains no private key"),
            #[cfg(feature = "tls")]
            TlsConfigError::InvalidKey(err) => write!(f, "key contains an invalid key, {}", err),
            #[cfg(feature = "tls-openssl")]
            TlsConfigError::EmptyCert => write!(f, "no public certificate found"),
            #[cfg(feature = "tls-openssl")]
            TlsConfigError::OpensslError(err) => write!(f, "Openssl failed, {}", err),
        }
    }
}

impl std::error::Error for TlsConfigError {}

/// Tls client authentication configuration.
pub(crate) enum TlsClientAuth {
    /// No client auth.
    Off,
    /// Allow any anonymous or authenticated client.
    Optional(Box<dyn Read + Send + Sync>),
    /// Allow any authenticated client.
    Required(Box<dyn Read + Send + Sync>),
}

/// Builder to set the configuration for the Tls server.
pub(crate) struct TlsConfigBuilder {
    cert: Box<dyn Read + Send + Sync>,
    key: Box<dyn Read + Send + Sync>,
    client_auth: TlsClientAuth,
    ocsp_resp: Vec<u8>,
}

impl fmt::Debug for TlsConfigBuilder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TlsConfigBuilder").finish()
    }
}

impl TlsConfigBuilder {
    /// Create a new TlsConfigBuilder
    pub(crate) fn new() -> TlsConfigBuilder {
        TlsConfigBuilder {
            key: Box::new(io::empty()),
            cert: Box::new(io::empty()),
            client_auth: TlsClientAuth::Off,
            ocsp_resp: Vec::new(),
        }
    }

    /// sets the Tls key via File Path, returns `TlsConfigError::IoError` if the file cannot be open
    pub(crate) fn key_path(mut self, path: impl AsRef<Path>) -> Self {
        self.key = Box::new(LazyFile {
            path: path.as_ref().into(),
            file: None,
        });
        self
    }

    /// sets the Tls key via bytes slice
    pub(crate) fn key(mut self, key: &[u8]) -> Self {
        self.key = Box::new(Cursor::new(Vec::from(key)));
        self
    }

    /// Specify the file path for the TLS certificate to use.
    pub(crate) fn cert_path(mut self, path: impl AsRef<Path>) -> Self {
        self.cert = Box::new(LazyFile {
            path: path.as_ref().into(),
            file: None,
        });
        self
    }

    /// sets the Tls certificate via bytes slice
    pub(crate) fn cert(mut self, cert: &[u8]) -> Self {
        self.cert = Box::new(Cursor::new(Vec::from(cert)));
        self
    }

    /// Sets the trust anchor for optional Tls client authentication via file path.
    ///
    /// Anonymous and authenticated clients will be accepted. If no trust anchor is provided by any
    /// of the `client_auth_` methods, then client authentication is disabled by default.
    pub(crate) fn client_auth_optional_path(mut self, path: impl AsRef<Path>) -> Self {
        let file = Box::new(LazyFile {
            path: path.as_ref().into(),
            file: None,
        });
        self.client_auth = TlsClientAuth::Optional(file);
        self
    }

    /// Sets the trust anchor for optional Tls client authentication via bytes slice.
    ///
    /// Anonymous and authenticated clients will be accepted. If no trust anchor is provided by any
    /// of the `client_auth_` methods, then client authentication is disabled by default.
    pub(crate) fn client_auth_optional(mut self, trust_anchor: &[u8]) -> Self {
        let cursor = Box::new(Cursor::new(Vec::from(trust_anchor)));
        self.client_auth = TlsClientAuth::Optional(cursor);
        self
    }

    /// Sets the trust anchor for required Tls client authentication via file path.
    ///
    /// Only authenticated clients will be accepted. If no trust anchor is provided by any of the
    /// `client_auth_` methods, then client authentication is disabled by default.
    pub(crate) fn client_auth_required_path(mut self, path: impl AsRef<Path>) -> Self {
        let file = Box::new(LazyFile {
            path: path.as_ref().into(),
            file: None,
        });
        self.client_auth = TlsClientAuth::Required(file);
        self
    }

    /// Sets the trust anchor for required Tls client authentication via bytes slice.
    ///
    /// Only authenticated clients will be accepted. If no trust anchor is provided by any of the
    /// `client_auth_` methods, then client authentication is disabled by default.
    pub(crate) fn client_auth_required(mut self, trust_anchor: &[u8]) -> Self {
        let cursor = Box::new(Cursor::new(Vec::from(trust_anchor)));
        self.client_auth = TlsClientAuth::Required(cursor);
        self
    }

    /// sets the DER-encoded OCSP response
    pub(crate) fn ocsp_resp(mut self, ocsp_resp: &[u8]) -> Self {
        self.ocsp_resp = Vec::from(ocsp_resp);
        self
    }
}

struct LazyFile {
    path: PathBuf,
    file: Option<File>,
}

impl LazyFile {
    fn lazy_read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.file.is_none() {
            self.file = Some(File::open(&self.path)?);
        }

        self.file.as_mut().unwrap().read(buf)
    }
}

impl Read for LazyFile {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.lazy_read(buf).map_err(|err| {
            let kind = err.kind();
            io::Error::new(
                kind,
                format!("error reading file ({:?}): {}", self.path.display(), err),
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_cert_key() {
        TlsConfigBuilder::new()
            .key_path("examples/tls/key.rsa")
            .cert_path("examples/tls/cert.pem")
            .build()
            .unwrap();
    }

    #[test]
    fn bytes_cert_key() {
        let key = include_str!("../examples/tls/key.rsa");
        let cert = include_str!("../examples/tls/cert.pem");

        TlsConfigBuilder::new()
            .key(key.as_bytes())
            .cert(cert.as_bytes())
            .build()
            .unwrap();
    }
}
