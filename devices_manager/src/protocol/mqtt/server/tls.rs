use std::fs::File;
use std::path::Path;
use tokio::net::TcpStream;

use serde::{Deserialize, Serialize};

use std::io::Read;

use crate::protocol::mqtt::server::broker::IO;
use tokio_rustls::rustls::{server::WebPkiClientVerifier, RootCertStore};
use {
    rustls_pemfile::Item,
    std::{io::BufReader, sync::Arc},
    tokio_rustls::rustls::{pki_types::PrivateKeyDer, Error as RustlsError, ServerConfig},
    tracing::error,
};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum TlsConfig {
    Rustls { capath: Option<String>, certpath: String, keypath: String },
}

impl TlsConfig {
    // Returns true only if all of the file paths inside `TlsConfig` actually exists on file system.
    // NOTE: This doesn't verify if certificate files are in required format or not.
    pub fn validate_paths(&self) -> bool {
        match self {
            TlsConfig::Rustls { capath, certpath, keypath } => {
                let ca = capath.is_none() || capath.as_ref().is_some_and(|v| Path::new(v).exists());

                ca && [certpath, keypath].iter().all(|v| Path::new(v).exists())
            }
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[error("Acceptor error")]
pub enum Error {
    #[error("I/O {0}")]
    Io(#[from] std::io::Error),
    #[error("No peer certificate")]
    NoPeerCertificate,
    #[error("Rustls error {0}")]
    Rustls(#[from] RustlsError),
    #[error("Server cert file {0} not found")]
    ServerCertNotFound(String),
    #[error("Invalid server cert file {0}")]
    InvalidServerCert(String),
    #[error("Invalid CA cert file {0}")]
    InvalidCACert(String),
    #[error("Invalid server key file {0}")]
    InvalidServerKey(String),
    #[error("Server private key file {0} not found")]
    ServerKeyNotFound(String),
    #[error("CA file {0} no found")]
    CaFileNotFound(String),
    RustlsNotEnabled,
    #[error("Invalid tenant id = {0}")]
    InvalidTenantId(String),
    #[error("Invalid tenant certificate")]
    InvalidTenant,
    #[error("Tenant id missing in certificate")]
    MissingTenantId,
    #[error("Tenant id missing in certificate")]
    CertificateParse,
}

/// Extract client id from certificate's subject common name field
fn extract_common_name(der: &[u8]) -> Result<Option<String>, Error> {
    let (_, cert) =
        x509_parser::parse_x509_certificate(der).map_err(|_| Error::CertificateParse)?;
    let client_id = match cert.subject().iter_common_name().next() {
        Some(org) => match org.as_str() {
            Ok(val) => val.to_string(),
            Err(_) => return Err(Error::InvalidTenant),
        },
        None => {
            return Ok(None);
        }
    };

    if client_id.chars().any(|c| !c.is_alphanumeric()) {
        return Err(Error::InvalidTenantId(client_id));
    }

    Ok(Some(client_id))
}

#[allow(dead_code)]
pub enum TLSAcceptor {
    Rustls { acceptor: tokio_rustls::TlsAcceptor },
}

impl TLSAcceptor {
    pub fn new(config: &TlsConfig) -> Result<Self, Error> {
        match config {
            TlsConfig::Rustls { capath, certpath, keypath } => {
                Self::rustls(capath, certpath, keypath)
            }
        }
    }

    pub async fn accept(&self, stream: TcpStream) -> Result<(Option<String>, Box<dyn IO>), Error> {
        match self {
            TLSAcceptor::Rustls { acceptor } => {
                let stream = acceptor.accept(stream).await?;

                let tenant_id = {
                    let (_, session) = stream.get_ref();
                    session
                        .peer_certificates()
                        .map(|cert| extract_common_name(&cert[0]))
                        .transpose()?
                        .flatten()
                };
                let network = Box::new(stream);
                Ok((tenant_id, network))
            }
        }
    }

    fn rustls(
        ca_path: &Option<String>,
        cert_path: &String,
        key_path: &String,
    ) -> Result<TLSAcceptor, Error> {
        let Some(ca_path) = ca_path else {
            return Err(Error::CaFileNotFound(
                "capath must be specified in config when verify-client-cert is enabled."
                    .to_string(),
            ));
        };

        let (certs, key) = {
            // Get certificates
            let cert_file = File::open(cert_path);
            let cert_file = cert_file.map_err(|_| Error::ServerCertNotFound(cert_path.clone()))?;
            let certs = rustls_pemfile::certs(&mut BufReader::new(cert_file))
                .collect::<Result<Vec<_>, _>>()
                .map_err(|_| Error::InvalidServerCert(cert_path.to_string()))?;

            // Get private key
            let key = first_private_key_in_pemfile(key_path)?;

            (certs, key)
        };

        let builder = ServerConfig::builder();

        // client authentication with a CA. CA isn't required otherwise
        let builder = {
            let ca_file = File::open(ca_path);
            let ca_file = ca_file.map_err(|_| Error::CaFileNotFound(ca_path.clone()))?;
            let ca_file = &mut BufReader::new(ca_file);
            let ca_cert = rustls_pemfile::certs(ca_file)
                .next()
                .ok_or_else(|| Error::InvalidCACert(ca_path.to_string()))??;

            let mut store = RootCertStore::empty();
            store.add(ca_cert).map_err(|_| Error::InvalidCACert(ca_path.to_string()))?;

            // This will only return an error if no trust anchors are provided or invalid CRLs are
            // provided. We always provide a trust anchor, and don't provide any CRLs, so it is safe
            // to unwrap.
            let verifier = WebPkiClientVerifier::builder(Arc::new(store)).build().unwrap();
            builder.with_no_client_auth()
        };

        let server_config = builder.with_single_cert(certs, key)?;

        let acceptor = tokio_rustls::TlsAcceptor::from(Arc::new(server_config));
        Ok(TLSAcceptor::Rustls { acceptor })
    }
}

/// Get the first private key in a PEM file
fn first_private_key_in_pemfile(key_path: &String) -> Result<PrivateKeyDer<'static>, Error> {
    // Get private key
    let key_file = File::open(key_path);
    let key_file = key_file.map_err(|_| Error::ServerKeyNotFound(key_path.clone()))?;

    let rd = &mut BufReader::new(key_file);

    // keep reading Items one by one to find a Key, return error if none found.
    loop {
        let item = rustls_pemfile::read_one(rd).map_err(|err| {
            error!("Error reading key file: {:?}", err);
            Error::InvalidServerKey(key_path.clone())
        })?;

        match item {
            Some(Item::Sec1Key(key)) => {
                return Ok(key.into());
            }
            Some(Item::Pkcs1Key(key)) => {
                return Ok(key.into());
            }
            Some(Item::Pkcs8Key(key)) => {
                return Ok(key.into());
            }
            None => {
                error!("No private key found in {:?}", key_path);
                return Err(Error::InvalidServerKey(key_path.clone()));
            }
            _ => {}
        }
    }
}
