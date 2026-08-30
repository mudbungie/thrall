//! **The mTLS half** (REMOTE §1.3, §4): the rustls configuration a foot dials
//! with, built from the material the operator carried here.
//!
//! **Certificates are the entire authentication story.** The engine requires a
//! client certificate that chains to the operator CA; this end requires the
//! same of the engine and presents its own. There is no password, token or
//! account anywhere in the channel — so there is nothing in it to phish, rotate
//! or leak, and a connection that cannot authenticate gets a **TLS refusal, not
//! a reply**: the handshake fails inside rustls and no byte of the boundary is
//! ever reached.
//!
//! **The provider is named, never defaulted.** `ClientConfig::builder()` reads
//! a process-global default and *panics* when none is installed or when two
//! are — a panic path, which prod does not have. Naming `ring` outright removes
//! the global read and the panic with it, and `ring` is the provider the
//! dependency approval fixed (`Cargo.toml`): rustls' defaults would select
//! `aws-lc-rs`, whose sys crate builds C and breaks the single-binary story
//! that is most of why a foot is easy to install on a box nobody administers
//! for you.

use std::path::Path;
use std::sync::Arc;

use rustls::pki_types::pem::PemObject;
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use rustls::{ClientConfig, RootCertStore};

use super::material::Material;

/// The foot's end: verify the engine against the operator CA, and present this
/// box's own leaf — the certificate that **is** this client's identity.
pub fn client_config(m: &Material) -> Result<Arc<ClientConfig>, String> {
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let (chain, key) = identity(&m.chain, &m.key)?;
    let config = ClientConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .map_err(|e| format!("tls versions: {e}"))?
        .with_root_certificates(anchors(&m.anchors)?)
        .with_client_auth_cert(chain, key)
        .map_err(|e| format!("{}: client identity: {e}", m.chain.display()))?;
    Ok(Arc::new(config))
}

/// The operator CA as a trust anchor store. Every certificate in the file is an
/// anchor: an operator who put two in meant two.
pub(crate) fn anchors(path: &Path) -> Result<RootCertStore, String> {
    let mut store = RootCertStore::empty();
    for anchor in
        CertificateDer::pem_file_iter(path).map_err(|e| format!("{}: {e}", path.display()))?
    {
        let anchor = anchor.map_err(|e| format!("{}: {e}", path.display()))?;
        store
            .add(anchor)
            .map_err(|e| format!("{}: {e}", path.display()))?;
    }
    if store.is_empty() {
        return Err(format!("{}: no certificate in it", path.display()));
    }
    Ok(store)
}

/// This end's chain and key, read from PEM.
pub(crate) fn identity(
    chain: &Path,
    key: &Path,
) -> Result<(Vec<CertificateDer<'static>>, PrivateKeyDer<'static>), String> {
    let certs: Vec<CertificateDer<'static>> = CertificateDer::pem_file_iter(chain)
        .map_err(|e| format!("{}: {e}", chain.display()))?
        .collect::<Result<_, _>>()
        .map_err(|e| format!("{}: {e}", chain.display()))?;
    if certs.is_empty() {
        return Err(format!("{}: no certificate in it", chain.display()));
    }
    let private =
        PrivateKeyDer::from_pem_file(key).map_err(|e| format!("{}: {e}", key.display()))?;
    Ok((certs, private))
}

#[cfg(test)]
mod tests;
