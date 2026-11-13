use std::sync::Arc;

use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::crypto::{CryptoProvider, verify_tls12_signature, verify_tls13_signature};
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use rustls::{ClientConfig, DigitallySignedStruct, RootCertStore, SignatureScheme};
use tokio_postgres_rustls::MakeRustlsConnect;

use crate::error::PgError;

pub(crate) fn make_tls_connector(
    verifies_certificates: bool,
) -> Result<MakeRustlsConnect, PgError> {
    Ok(MakeRustlsConnect::new(client_config(
        verifies_certificates,
    )?))
}

fn client_config(verifies_certificates: bool) -> Result<ClientConfig, PgError> {
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let builder = ClientConfig::builder_with_provider(provider.clone())
        .with_safe_default_protocol_versions()
        .map_err(|error| PgError::Connect(error.to_string()))?;
    let client_config = if verifies_certificates {
        let mut root_store = RootCertStore::empty();
        root_store.add_parsable_certificates(rustls_native_certs::load_native_certs().certs);
        builder
            .with_root_certificates(root_store)
            .with_no_client_auth()
    } else {
        builder
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(AcceptAnyCertificate { provider }))
            .with_no_client_auth()
    };
    Ok(client_config)
}

#[derive(Debug)]
struct AcceptAnyCertificate {
    provider: Arc<CryptoProvider>,
}

impl ServerCertVerifier for AcceptAnyCertificate {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        certificate: &CertificateDer<'_>,
        signature: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        verify_tls12_signature(
            message,
            certificate,
            signature,
            &self.provider.signature_verification_algorithms,
        )
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        certificate: &CertificateDer<'_>,
        signature: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        verify_tls13_signature(
            message,
            certificate,
            signature,
            &self.provider.signature_verification_algorithms,
        )
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.provider
            .signature_verification_algorithms
            .supported_schemes()
    }
}

#[cfg(test)]
mod tests {
    use rustls::pki_types::PrivateKeyDer;
    use rustls::{ClientConnection, ProtocolVersion, ServerConfig, ServerConnection};

    use super::*;

    const CERTIFICATE_DER: &[u8] = include_bytes!("../tests/fixtures/localhost-cert.der");
    const PRIVATE_KEY_DER: &[u8] = include_bytes!("../tests/fixtures/localhost-key.der");

    fn self_signed_server(
        protocol_versions: &[&'static rustls::SupportedProtocolVersion],
    ) -> ServerConfig {
        ServerConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
            .with_protocol_versions(protocol_versions)
            .unwrap()
            .with_no_client_auth()
            .with_single_cert(
                vec![CertificateDer::from(CERTIFICATE_DER.to_vec())],
                PrivateKeyDer::try_from(PRIVATE_KEY_DER.to_vec()).unwrap(),
            )
            .unwrap()
    }

    fn transfer(
        source: &mut impl std::ops::DerefMut<Target = rustls::ConnectionCommon<impl Sized>>,
        destination: &mut impl std::ops::DerefMut<Target = rustls::ConnectionCommon<impl Sized>>,
    ) {
        let mut buffer = Vec::new();
        while source.wants_write() {
            source.write_tls(&mut buffer).unwrap();
        }
        let mut pending = buffer.as_slice();
        while !pending.is_empty() {
            destination.read_tls(&mut pending).unwrap();
        }
    }

    fn handshake(
        verifies_certificates: bool,
        server_config: ServerConfig,
    ) -> Result<Option<ProtocolVersion>, rustls::Error> {
        let mut client = ClientConnection::new(
            Arc::new(client_config(verifies_certificates).unwrap()),
            ServerName::try_from("localhost").unwrap(),
        )?;
        let mut server = ServerConnection::new(Arc::new(server_config)).unwrap();
        for _ in 0..10 {
            if !client.is_handshaking() && !server.is_handshaking() {
                return Ok(client.protocol_version());
            }
            transfer(&mut client, &mut server);
            server.process_new_packets().unwrap();
            transfer(&mut server, &mut client);
            client.process_new_packets()?;
        }
        panic!("handshake did not finish");
    }

    #[test]
    fn non_verifying_config_accepts_a_self_signed_certificate_over_tls13() {
        let negotiated_version = handshake(false, self_signed_server(&[&rustls::version::TLS13]));
        assert_eq!(negotiated_version, Ok(Some(ProtocolVersion::TLSv1_3)));
    }

    #[test]
    fn non_verifying_config_accepts_a_self_signed_certificate_over_tls12() {
        let negotiated_version = handshake(false, self_signed_server(&[&rustls::version::TLS12]));
        assert_eq!(negotiated_version, Ok(Some(ProtocolVersion::TLSv1_2)));
    }

    #[test]
    fn verifying_config_rejects_a_self_signed_certificate() {
        let handshake_error = handshake(true, self_signed_server(&[&rustls::version::TLS13]))
            .expect_err("unknown issuer is rejected");
        assert_eq!(
            handshake_error,
            rustls::Error::InvalidCertificate(rustls::CertificateError::UnknownIssuer)
        );
    }
}
