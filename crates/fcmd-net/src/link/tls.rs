//! The two TLS 1.3 configurations of the Link (SPEC 04 §9.4).
//!
//! Each side shows its own self-signed certificate and asks for the other's. No authority
//! vouches for either, so the verifiers below accept any certificate *whose key signs the
//! handshake* — the signature is checked, so a side cannot show a certificate it has not the
//! key of — and leave the decision to the fingerprint, which the caller of these functions
//! compares with the one it knows, or shows as six digits the first time.

use std::sync::Arc;

use tokio_rustls::rustls::client::danger::{
    HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier,
};
use tokio_rustls::rustls::crypto::{ring as provider, verify_tls13_signature, CryptoProvider};
use tokio_rustls::rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use tokio_rustls::rustls::server::danger::{ClientCertVerified, ClientCertVerifier};
use tokio_rustls::rustls::{
    version::TLS13, ClientConfig, DigitallySignedStruct, DistinguishedName, Error, ServerConfig,
    SignatureScheme,
};

use super::identity::Identity;

/// The name a caller asks for: the same for every machine, since none is found by name.
pub const SERVER_NAME: &str = "finalcommander-link";

fn crypto() -> Arc<CryptoProvider> {
    Arc::new(provider::default_provider())
}

/// What the called side answers with: its certificate, and a demand for the caller's.
pub fn server_config(identity: &Identity) -> Result<Arc<ServerConfig>, Error> {
    let crypto = crypto();
    let config = ServerConfig::builder_with_provider(crypto.clone())
        .with_protocol_versions(&[&TLS13])?
        .with_client_cert_verifier(Arc::new(AnyKey(crypto)))
        .with_single_cert(vec![identity.certificate()], identity.private_key())?;
    Ok(Arc::new(config))
}

/// What the caller connects with: its certificate, and any certificate from the other side
/// whose key signs the handshake.
pub fn client_config(identity: &Identity) -> Result<Arc<ClientConfig>, Error> {
    let crypto = crypto();
    let config = ClientConfig::builder_with_provider(crypto.clone())
        .with_protocol_versions(&[&TLS13])?
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(AnyKey(crypto)))
        .with_client_auth_cert(vec![identity.certificate()], identity.private_key())?;
    Ok(Arc::new(config))
}

/// The name to connect with.
pub fn server_name() -> ServerName<'static> {
    ServerName::try_from(SERVER_NAME).expect("a valid name")
}

/// Accepts any certificate, and checks that the handshake is signed with its key.
#[derive(Debug)]
struct AnyKey(Arc<CryptoProvider>);

impl AnyKey {
    fn tls13(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, Error> {
        verify_tls13_signature(
            message,
            cert,
            dss,
            &self.0.signature_verification_algorithms,
        )
    }

    fn schemes(&self) -> Vec<SignatureScheme> {
        self.0.signature_verification_algorithms.supported_schemes()
    }
}

impl ServerCertVerifier for AnyKey {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, Error> {
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, Error> {
        // TLS 1.3 only: never reached.
        Err(Error::General("TLS 1.2 is not spoken here".into()))
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, Error> {
        self.tls13(message, cert, dss)
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.schemes()
    }
}

impl ClientCertVerifier for AnyKey {
    fn root_hint_subjects(&self) -> &[DistinguishedName] {
        &[]
    }

    fn verify_client_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _now: UnixTime,
    ) -> Result<ClientCertVerified, Error> {
        Ok(ClientCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, Error> {
        Err(Error::General("TLS 1.2 is not spoken here".into()))
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, Error> {
        self.tls13(message, cert, dss)
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.schemes()
    }
}
