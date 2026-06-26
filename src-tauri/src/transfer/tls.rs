use anyhow::{Context, Result};
use std::path::Path;
use std::sync::Arc;

pub type TlsConfig = Arc<rustls::ServerConfig>;

pub fn load_or_generate_tls_config(data_dir: &Path) -> Result<TlsConfig> {
    let tls_dir = data_dir.join("tls");
    std::fs::create_dir_all(&tls_dir).ok();
    let cert_path = tls_dir.join("cert.pem");
    let key_path = tls_dir.join("key.pem");

    if !cert_path.exists() || !key_path.exists() {
        let (cert_pem, key_pem) = generate_self_signed()?;
        std::fs::write(&cert_path, &cert_pem).context("write cert")?;
        std::fs::write(&key_path, &key_pem).context("write key")?;
    }

    let cert_pem = std::fs::read_to_string(&cert_path).context("read cert")?;
    let key_pem = std::fs::read_to_string(&key_path).context("read key")?;
    load_config(&cert_pem, &key_pem)
}

fn generate_self_signed() -> Result<(String, String)> {
    use rcgen::{CertificateParams, KeyPair};
    let key = KeyPair::generate()?;
    let mut params = CertificateParams::new(["sendsent".into()])?;
    params.distinguished_name = rcgen::DistinguishedName::new();
    params.distinguished_name.push(rcgen::DnType::CommonName, "SendSent TLS");
    let cert = params.self_signed(&key)?;
    Ok((cert.pem(), key.serialize_pem()))
}

fn load_config(cert_pem: &str, key_pem: &str) -> Result<TlsConfig> {
    use rustls::pki_types::{CertificateDer, PrivateKeyDer};
    let certs: Vec<CertificateDer<'static>> = rustls_pemfile::certs(&mut cert_pem.as_bytes())
        .collect::<std::io::Result<Vec<_>>>()
        .context("parse cert")?;
    let key: PrivateKeyDer<'static> = rustls_pemfile::private_key(&mut key_pem.as_bytes())
        .context("parse key")?
        .context("no private key")?;
    let config = rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, key)
        .context("build tls config")?;
    Ok(Arc::new(config))
}

pub fn make_client_config() -> Arc<rustls::ClientConfig> {
    let config = rustls::ClientConfig::builder()
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(NoVerify))
        .with_no_client_auth();
    Arc::new(config)
}

#[derive(Debug)]
struct NoVerify;

impl rustls::client::danger::ServerCertVerifier for NoVerify {
    fn verify_server_cert(
        &self,
        _end_entity: &rustls::pki_types::CertificateDer<'_>,
        _intermediates: &[rustls::pki_types::CertificateDer<'_>],
        _server_name: &rustls::pki_types::ServerName<'_>,
        _ocsp_response: &[u8],
        _now: rustls::pki_types::UnixTime,
    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        rustls::crypto::ring::default_provider()
            .signature_verification_algorithms
            .supported_schemes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Once;

    static INIT_RING: Once = Once::new();
    fn ensure_ring() { INIT_RING.call_once(|| rustls::crypto::ring::default_provider().install_default().unwrap()); }

    #[test]
    fn gen_load_roundtrip() {
        ensure_ring();
        let tmp = std::env::temp_dir().join(format!("ss-tls-{}", uuid::Uuid::new_v4()));
        let _cfg = load_or_generate_tls_config(&tmp).unwrap();
        let _cfg2 = load_or_generate_tls_config(&tmp).unwrap();
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn client_config_constructs() {
        ensure_ring();
        let _ = make_client_config();
    }
}
