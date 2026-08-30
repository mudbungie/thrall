//! **The operator's act, performed by the suite** (REMOTE §1.4, DESIGN §3.3).
//!
//! thrall mints nothing. A certificate and its key are issued by the operator's
//! own CA on the box that holds it and carried to the foot by hand — so there
//! is no production caller here, and there must never be one: a foot that could
//! provision itself over the wire would be a foot any wire could provision.
//! What the suite needs is a channel to test, and the honest way to get one is
//! to do out of band exactly what the operator does out of band.
//!
//! It shells to `openssl`, which is the tool an operator would use, through the
//! crate's one spawn boundary. Nothing is committed: a fixture key in a tree is
//! a private key in a repository.
//!
//! **What each leaf says**, and every fact here is REMOTE's rather than this
//! file's:
//!
//! - the **engine** leaf carries `serverAuth` and a SAN naming loopback, which
//!   is what a client verifies against what it dialled;
//! - the **foot** leaf carries `clientAuth` and `OU=foot` — REMOTE §4.2's
//!   grade, written by the operator's own CA because that is the only party
//!   entitled to write it;
//! - the **operator** leaf is the same thing without the organizational unit,
//!   which is what a leaf minted before the grade existed looks like. It exists
//!   so the suite can prove thrall refuses to run as one.

use std::path::Path;

/// The CA's own key. It never leaves the scratch directory, and no production
/// path knows the name.
const CA_KEY: &str = "ca.key";
/// The curve every key is drawn on.
const CURVE: &str = "ec_paramgen_curve:P-256";
/// How long a minted certificate is good for. A day, because the longest thing
/// the suite does with one is finish.
const DAYS: &str = "1";
/// The organizational unit that is the foot grade (REMOTE §4.2).
pub(crate) const FOOT: &str = "foot";
/// The common name the suite's foot presents.
pub(crate) const FOOT_NAME: &str = "thrall-test-foot";
/// The common name of the leaf that is NOT a foot.
pub(crate) const OPERATOR_NAME: &str = "thrall-test-operator";
/// The basename of the leaf that says foot and names no client at all — a
/// subject with the grade and no common name, which is a certificate that
/// authenticates nobody.
pub(crate) const NAMELESS: &str = "nameless";
/// The engine leaf's basename and common name.
pub(crate) const ENGINE: &str = "engine";

/// Mint a CA and the three leaves into `dir`, in the layout
/// [`material`](crate::channel::material) reads: the foot's pair is written as
/// `client.pem`/`client.key`, which is the entry spelling REMOTE §8.2 fixes.
pub(crate) fn material(dir: &Path) {
    std::fs::create_dir_all(dir).expect("the scratch directory");
    ca(dir);
    leaf(
        dir,
        ENGINE,
        &format!("/CN={ENGINE}"),
        "IP:127.0.0.1",
        "serverAuth",
    );
    leaf(
        dir,
        "client",
        &format!("/OU={FOOT}/CN={FOOT_NAME}"),
        &format!("DNS:{FOOT_NAME}"),
        "clientAuth",
    );
    leaf(
        dir,
        OPERATOR_NAME,
        &format!("/CN={OPERATOR_NAME}"),
        &format!("DNS:{OPERATOR_NAME}"),
        "clientAuth",
    );
    leaf(
        dir,
        NAMELESS,
        &format!("/OU={FOOT}"),
        &format!("DNS:{NAMELESS}"),
        "clientAuth",
    );
}

/// [`material`], plus the address file — the whole of what one provisioned
/// entry holds, answered as the material a channel opens from. The tests that
/// stand an [`Engine`](super::engine::Engine) up do not use it: an engine binds
/// a kernel-chosen port and writes the address itself, because only the
/// listener knows what `:0` became.
pub(crate) fn provisioned(dir: &Path, address: &str) -> crate::channel::material::Material {
    material(dir);
    std::fs::write(dir.join(crate::channel::material::ADDRESS), address).expect("the address");
    crate::channel::material::read_dir(dir)
        .expect("readable")
        .expect("provisioned")
}

/// The self-signed operator CA both ends verify against.
fn ca(dir: &Path) {
    tool(&[
        "req",
        "-x509",
        "-newkey",
        "ec",
        "-pkeyopt",
        CURVE,
        "-nodes",
        "-sha256",
        "-days",
        DAYS,
        "-subj",
        "/CN=thrall-test-ca",
        "-keyout",
        &path(dir, CA_KEY),
        "-out",
        &path(dir, crate::channel::material::ANCHORS),
    ]);
}

/// One leaf: a key and a bare request, then the signature that carries the two
/// facts the issuer decides — the subject alternative name and the extended key
/// usage.
///
/// The extensions travel in a file rather than through `req -addext` with
/// `x509 -copy_extensions`, which is OpenSSL-only: LibreSSL ships as `openssl`
/// on macOS and refuses that flag outright. `-extfile`/`-extensions` is the
/// spelling both toolsets have, and it is the more honest model besides — what
/// a certificate asserts is decided by whoever signs it, not by whoever asked.
fn leaf(dir: &Path, name: &str, subject: &str, san: &str, eku: &str) {
    let ext = dir.join(format!("{name}.ext"));
    std::fs::write(
        &ext,
        format!("[leaf]\nsubjectAltName={san}\nextendedKeyUsage={eku}\n"),
    )
    .expect("the extension file");
    tool(&[
        "req",
        "-new",
        "-newkey",
        "ec",
        "-pkeyopt",
        CURVE,
        "-nodes",
        "-sha256",
        "-subj",
        subject,
        "-keyout",
        &path(dir, &format!("{name}.key")),
        "-out",
        &path(dir, &format!("{name}.csr")),
    ]);
    tool(&[
        "x509",
        "-req",
        "-sha256",
        "-days",
        DAYS,
        "-extfile",
        &ext.to_string_lossy(),
        "-extensions",
        "leaf",
        "-in",
        &path(dir, &format!("{name}.csr")),
        "-CA",
        &path(dir, crate::channel::material::ANCHORS),
        "-CAkey",
        &path(dir, CA_KEY),
        // LibreSSL refuses to sign with no serial file unless told it may make
        // one; OpenSSL 3 accepts the flag and does the same thing.
        "-CAcreateserial",
        "-out",
        &path(dir, &format!("{name}.pem")),
    ]);
}

/// One path, as the string `openssl` takes.
fn path(dir: &Path, leaf: &str) -> String {
    dir.join(leaf).to_string_lossy().into_owned()
}

/// One `openssl` run, through the crate's one spawn boundary. A failure is the
/// scaffolding's own, so it dies here with the tool's sentence rather than
/// becoming a confusing refusal three layers up.
fn tool(args: &[&str]) {
    let mut cmd = crate::spawn::command("openssl");
    cmd.args(args);
    let said = crate::spawn::output(&mut cmd).expect("openssl is installed");
    assert!(
        said.status.success(),
        "openssl {args:?}: {}",
        String::from_utf8_lossy(&said.stderr)
    );
}
