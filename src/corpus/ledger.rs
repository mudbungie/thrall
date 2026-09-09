//! **The engine's shape ledger, vendored** (yog's `docs/REMOTE.md` §3.2) — the
//! edition every field path of every shape a foot speaks appeared at.
//!
//! §3.2's ruling: `PROTOCOL` is a **major**, moved only by a breaking change,
//! and every *addition* — a field, a word, an op, a reply kind — ships with no
//! bump and is stamped an **edition** instead. The old integer line continued,
//! so the stamps 1..18 in [`shapes`] are the versions this crate already
//! vendored; what is new is that they are per PATH rather than per shape, and
//! that they are read rather than recited.
//!
//! **[`FLOOR`] is what makes a stamp actionable.** It is the edition the
//! current major was cut at: every path stamped at or below it is present on
//! every engine speaking this major, so a reader may require it — which is what
//! every reader here does today. Every path stamped ABOVE it is optional to
//! read, because an engine of an older edition does not write it, and that is
//! the one class this crate has no member of yet (its newest stamp is 8, ten
//! editions under the floor). The replay in [`replay`](super::replay) is what
//! keeps that true rather than accidental.
//!
//! **It is `.rs` rather than yog's `.json`, for [`corpus`](super)'s reason**: a
//! vendored JSON file would have to be ruled into `Cargo.toml`'s `include`
//! allowlist to survive a build from the registry, and would then ship a test
//! fixture inside a released crate. The text is copied out of
//! `corpus/shapes.json`, key for key, so a re-vendor is a diff.

/// The vendored table itself.
mod shapes;

pub(crate) use shapes::SHAPES;

/// **The edition the current major was cut at** (`shapes.json`'s `floor`).
///
/// A path stamped at or below it is required; above it, optional to read. It
/// moves in the one regeneration that raises `PROTOCOL`, and never after.
pub(crate) const FLOOR: u32 = 18;

/// **What yog has listed for removal** (`shapes.json`'s `deprecated`), which is
/// nothing. §3.2's window rule: a path is removed only at a major, only after
/// being listed here in a *published* release, and never within three releases
/// of that listing — so a consumer that reads this list is told before a field
/// it depends on can go.
pub(crate) const DEPRECATED: [&str; 0] = [];

/// One vendored shape: its name in `corpus/`, and every field path it carries
/// stamped with the edition that path appeared at.
///
/// A path is spelled exactly as `shapes.json` spells it — `"<path>:<type>"`,
/// segments separated by `/`, `[]` for every element of an array, the empty
/// path for the frame itself — because a re-spelling is a second copy of the
/// engine's fact and this file exists to hold exactly one.
pub(crate) struct Shape {
    /// The corpus name, `request/<op>` or `reply/<kind>`.
    pub(crate) name: &'static str,
    /// Every path, stamped, sorted as `shapes.json` sorts them.
    pub(crate) signature: &'static [(&'static str, u32)],
}

/// **This vendored corpus's edition: its newest stamp**, computed and never
/// stored (§3.2). It is what [`hello::EDITION`](crate::channel::hello::EDITION)
/// states on the wire, and the two agreeing is a test rather than a tautology —
/// the same discipline the protocol pin keeps (`corpus::tests`).
///
/// **It is 8 rather than the engine's 18, and that is honest.** An edition is a
/// statement about what this end can READ, and a foot reads nothing stamped
/// past 8: every field added since is on a seat-facing shape it never decodes.
/// A field added at 14 to a shape this crate has no decoder for cannot make
/// this crate newer.
pub(crate) fn edition() -> u32 {
    SHAPES
        .iter()
        .flat_map(|shape| shape.signature.iter().map(|&(_, since)| since))
        .max()
        .unwrap_or(FLOOR)
}

/// One shape's signature, by name. `None` is a shape this foot does not speak.
pub(crate) fn signature(name: &str) -> Option<&'static [(&'static str, u32)]> {
    SHAPES
        .iter()
        .find(|shape| shape.name == name)
        .map(|shape| shape.signature)
}

/// **The paths of one JSON type**, spelled without the type suffix.
///
/// The mutation replay asks for `"string"`: every place a frame carries a word,
/// which is where a vocabulary that grew a word this build has not heard of
/// would arrive.
pub(crate) fn paths_of_type(signature: &[(&'static str, u32)], kind: &str) -> Vec<&'static str> {
    let suffix = format!(":{kind}");
    signature
        .iter()
        .filter_map(|&(entry, _)| entry.strip_suffix(&suffix))
        .collect()
}

/// **The path an entry names**, with its type suffix cut off — the half a
/// projector walks a frame by.
pub(crate) fn path_of(entry: &str) -> &str {
    entry.rsplit_once(':').map_or(entry, |(path, _)| path)
}

#[cfg(test)]
mod tests;
