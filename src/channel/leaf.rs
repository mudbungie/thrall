//! **The foot grade, read off this box's own certificate** (REMOTE §4.2,
//! DESIGN §3.2).
//!
//! REMOTE §4.2 gives the grade one home — *"the subject's organizational unit,
//! read by the walk that already reads the common name: `CN=<client>, OU=foot`
//! is a foot"* — and one issuer: the operator's own CA, out of channel, on the
//! same act §1.4 already requires. **Enforcement is the server's**, because the
//! server is the party that can be trusted to enforce it. thrall's obligation
//! is the half it can keep: carry a leaf of that grade, and refuse to be
//! configured with anything else.
//!
//! **So this end fails closed where the engine fails open, and the asymmetry is
//! the design.** yog reads a subject with no `OU=foot` as operator grade —
//! default-operator, so a certificate minted before the grade existed keeps
//! working. thrall refuses that same certificate: an operator-grade leaf on a
//! foot is a machine holding the whole boundary in order to run commands, which
//! is the thing being a foot exists to give up. Bytes that are not a
//! certificate at all refuse for the same reason. Neither end is defaulting;
//! they are answering different questions.
//!
//! **thrall links no certificate library** (`Cargo.toml`'s approved set), so
//! this is a DER walk — structural ASN.1 rather than a byte search, and the
//! structure is the point: the **issuer** carries a common name too, and it
//! comes FIRST, so a scan for the common-name object identifier would answer
//! the operator CA's name for every leaf on the box.
//!
//! What it reads, per RFC 5280:
//!
//! ```text
//! Certificate     ::= SEQUENCE { tbsCertificate, signatureAlgorithm, signature }
//! TBSCertificate  ::= SEQUENCE { [0] version OPTIONAL, serialNumber INTEGER,
//!                                signature, issuer, validity, subject, … }
//! Name            ::= SEQUENCE OF SET OF SEQUENCE { type OID, value ANY }
//! ```
//!
//! The optional `[0] version` is why `subject` is located **relative to the
//! serial number** rather than at a fixed index: the serial is the first field
//! certainly present, and `subject` is four constructed values past it. A
//! version-1 certificate and a version-3 one then take one path, not two.

use std::path::Path;

use rustls::pki_types::CertificateDer;
use rustls::pki_types::pem::PemObject;

/// DER tags this walk names.
const INTEGER: u8 = 0x02;
const OID: u8 = 0x06;
/// `id-at-commonName` — ASN.1 `{joint-iso-itu-t(2) ds(5) attributeType(4)
/// commonName(3)}`, in its DER encoding. Spelled as bytes rather than as the
/// dotted arc string because the dotted form of four small arcs is
/// indistinguishable from an IPv4 address, to a reader and to `make leak-scan`.
const COMMON_NAME: [u8; 3] = [0x55, 0x04, 0x03];
/// `id-at-organizationalUnitName` — the same arc one attribute over, and the
/// home REMOTE §4.2 gives the grade.
const ORG_UNIT: [u8; 3] = [0x55, 0x04, 0x0b];
/// The organizational unit that says foot. One word, written by the operator's
/// CA or not at all.
const FOOT: &str = "foot";
/// How many constructed fields separate `serialNumber` from `subject`:
/// signature, issuer, validity, subject.
const SERIAL_TO_SUBJECT: usize = 4;

/// **This box's identity on this channel, if it is entitled to be a foot.**
///
/// Answers the leaf's subject common name — which *is* the client identity the
/// engine reads back off the presented certificate (REMOTE §2), so a foot that
/// knows its own name learned it from the same bytes the engine will. Refuses
/// anything else, naming what it read.
pub fn foot(chain: &Path) -> Result<String, String> {
    let der = first(chain)?;
    let Some(name) = common_name(&der) else {
        return Err(format!(
            "{}: this leaf states no common name, so it names no client \
             (REMOTE §2: one certificate is one client identity)",
            chain.display()
        ));
    };
    if !is_foot(&der) {
        return Err(format!(
            "{}: the leaf {name:?} is not foot grade — a thrall carries a \
             certificate whose subject says OU={FOOT} and nothing else \
             (REMOTE §4.2). Mint one on the box that holds the CA.",
            chain.display()
        ));
    }
    Ok(name)
}

/// The first certificate in a PEM chain, as DER. The leaf comes first by
/// convention and by every mint's own order; what follows it is the chain to
/// the anchor, and none of it is this box's identity.
///
/// **The refusal says what the bytes should have been** (bl-52ba). This is what
/// a mis-copied file looks like, and it is the one refusal in this file that
/// used to say only what the PEM reader said — *"no items found"* — next to a
/// sibling one line away that names the grade, the section that fixes it and
/// the act that mints one. A file that locates a problem and stops there leaves
/// the operator exactly where they started.
fn first(chain: &Path) -> Result<Vec<u8>, String> {
    let held = CertificateDer::from_pem_file(chain).map_err(|e| {
        format!(
            "{}: no certificate could be read here — a thrall's {} holds the leaf \
             its operator issued for this channel, PEM-encoded, with the chain to \
             the anchor after it (REMOTE §8.2). {} ({e})",
            chain.display(),
            super::material::CHAIN,
            super::material::REMEDY
        )
    })?;
    Ok(held.to_vec())
}

/// The subject common name of a DER-encoded certificate, or `None` when the
/// bytes are not a certificate or carry no readable one.
///
/// The **last** common name wins. A distinguished name is written most-general
/// first in DER and most-specific last (RFC 4514 renders it reversed), so the
/// final one is the leaf's own.
fn common_name(der: &[u8]) -> Option<String> {
    attributes(subject(der)?, COMMON_NAME).pop()
}

/// Whether the same subject says foot.
fn is_foot(der: &[u8]) -> bool {
    subject(der).is_some_and(|name| attributes(name, ORG_UNIT).iter().any(|unit| unit == FOOT))
}

/// The `Name` bytes of the certificate's **subject** — located relative to the
/// serial number, for the reason the module doc gives.
fn subject(der: &[u8]) -> Option<&[u8]> {
    let (_, certificate, _) = tlv(der)?;
    let (_, tbs, _) = tlv(certificate)?;
    let fields = elements(tbs);
    let serial = fields.iter().position(|(tag, _)| *tag == INTEGER)?;
    let &(_, subject) = fields.get(serial + SERIAL_TO_SUBJECT)?;
    Some(subject)
}

/// Every value of attribute `oid` in a `Name`, in DER order, decoded as UTF-8.
/// Every string type these attributes are minted in — `UTF8String`,
/// `PrintableString`, `IA5String` — is UTF-8 or a subset of it, and one that is
/// not (`BMPString` is UTF-16) fails the decode and is skipped rather than
/// mis-read.
fn attributes(name: &[u8], oid: [u8; 3]) -> Vec<String> {
    let mut found = Vec::new();
    for (_, rdn) in elements(name) {
        for (_, attribute) in elements(rdn) {
            let parts = elements(attribute);
            let (Some(&(tag, kind)), Some(&(_, value))) = (parts.first(), parts.get(1)) else {
                continue;
            };
            if tag != OID || kind != oid {
                continue;
            }
            if let Ok(text) = std::str::from_utf8(value) {
                found.push(text.to_owned());
            }
        }
    }
    found
}

/// One DER type-length-value off the front of `bytes`: its tag, its contents,
/// and what follows it. `None` for a truncated header, a truncated value, or a
/// length DER does not permit — the indefinite form (`0x80`), which BER allows
/// and DER forbids, and a length wider than this walk will serve.
fn tlv(bytes: &[u8]) -> Option<(u8, &[u8], &[u8])> {
    let tag = *bytes.first()?;
    let first = *bytes.get(1)?;
    let (len, header) = if first < 0x80 {
        (usize::from(first), 2)
    } else {
        let width = usize::from(first & 0x7f);
        if width == 0 || width > 4 {
            return None;
        }
        let mut len: usize = 0;
        for i in 0..width {
            len = (len << 8) | usize::from(*bytes.get(2 + i)?);
        }
        (len, 2 + width)
    };
    // Saturating rather than checked: an unreachable overflow arm is an
    // untestable branch, and a saturated end simply fails the read below.
    let end = header.saturating_add(len);
    let value = bytes.get(header..end)?;
    Some((tag, value, bytes.get(end..).unwrap_or_default()))
}

/// Every element of a constructed value, in order. A trailing byte run that is
/// not a whole TLV ends the walk — a malformed tail yields the elements read
/// before it, which is what makes every read above total.
fn elements(mut body: &[u8]) -> Vec<(u8, &[u8])> {
    let mut out = Vec::new();
    while let Some((tag, value, rest)) = tlv(body) {
        out.push((tag, value));
        body = rest;
    }
    out
}

#[cfg(test)]
mod tests;
