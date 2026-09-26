//! **Why a channel could not carry a gesture** (DESIGN §3.8) — the two
//! classes that differ in what to do next, kept apart from the channel that
//! produces them because a classification is not a conversation.

/// **Why a channel could not carry a gesture** — in the two classes that
/// differ in what to do next, and in nothing else.
///
/// The split is drawn from *what failed*, never from the sentence: a foot that
/// decided its own lifetime by reading prose would be a foot the far end could
/// rewrite by rewording.
#[derive(Debug, PartialEq, Eq)]
pub enum Failure {
    /// **The transport.** It carries no opinion of either binary — a socket
    /// that would not open, an engine that went away, a peer that hung up
    /// before it had said anything — so the same ask down a fresh connection
    /// may well land.
    Wire(String),
    /// **The two ends do not speak one protocol version.** REMOTE §3 admits no
    /// negotiation and names the remedy in the sentence itself — *"upgrade the
    /// older component"* — so this is the one failure a further dial cannot
    /// improve: only a new binary on one of the two boxes can, and until one
    /// arrives every dial buys the same handshake and the same sentence.
    Skew(String),
}
