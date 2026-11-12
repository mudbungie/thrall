//! **The registration URL** (REMOTE §15.1): the whole of what travels to a
//! new device — `https://<host>[:port]/yog/register/<code>`. It carries no
//! key and no certificate, so reading it is reading three facts: where to
//! connect, what to call the server there, and the path to post to.

/// The port an `https` URL names when it names none.
const HTTPS: &str = "443";
/// The path every registration URL has, up to its code.
const DOOR: &str = "yog/register/";

/// Where one offer is claimed.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct Door {
    /// `host:port` — what is connected to, and what the certificate is
    /// verified against.
    pub(super) address: String,
    /// The URL's own authority, for the `Host` header.
    pub(super) host: String,
    /// The path the claim is posted to.
    pub(super) path: String,
}

/// Read `url` as a registration URL, or say what one looks like.
///
/// **Every part is judged before a byte is sent**, because the three facts
/// are written into a request line and a header: the code is letters and
/// digits and nothing else, and the authority is a name or an address the
/// TLS layer itself would accept, with a port that is a number.
pub(super) fn parse(url: &str) -> Result<Door, String> {
    read(url).ok_or_else(|| {
        format!(
            "{url:?} is not a registration URL. One reads \
             https://<host>[:port]/yog/register/<code>, exactly as the seat that \
             minted the offer showed it."
        )
    })
}

/// The three facts, where `url` states all of them.
fn read(url: &str) -> Option<Door> {
    let (authority, path) = url.strip_prefix("https://")?.split_once('/')?;
    let code = path.strip_prefix(DOOR)?;
    let ported = authority
        .rsplit_once(':')
        .is_some_and(|(_, port)| port.parse::<u16>().is_ok());
    let implied = format!("{authority}:{HTTPS}");
    let address = if ported {
        authority.to_owned()
    } else {
        implied
    };
    let coded = !code.is_empty() && code.chars().all(|c| c.is_ascii_alphanumeric());
    let named = crate::channel::server_name(&address).is_ok();
    (coded && named).then(|| Door {
        address,
        host: authority.to_owned(),
        path: format!("/{path}"),
    })
}

#[cfg(test)]
mod tests;
