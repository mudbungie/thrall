//! **The three vendored numbers**, and the whole of what this build states
//! about its own version (yog's `docs/REMOTE.md` §3.2).
//!
//! They are separated from the exchange in [`hello`](super) because they are a
//! different kind of thing: the exchange is this crate's behaviour, and these
//! are the ENGINE's facts, copied. A re-vendor rewrites this file and nothing
//! beside it.

/// The protocol this build speaks, and it is a **major** (REMOTE §3.2).
///
/// **A new verb is not a bump, and neither is a new field or a new word.**
/// Since bl-e598 the integer moves only on a BREAK — a field removed or
/// re-typed, a meaning changed under a spelling still in use, a field the
/// engine newly *requires* on a request. Every addition ships unbumped and is
/// stamped an **edition** in the corpus ledger instead ([`EDITION`]), which is
/// the fact this preface now carries beside the number.
///
/// **The equality is unchanged**: strict, fail-closed, both numbers named, no
/// negotiation. What changed is how rarely it can be reached, and therefore how
/// much a mismatch means when it is. 18 → 19 was the last bump of the old kind
/// and the first of the new.
///
/// **The number is the engine's and there is nothing here to version.** It must
/// read whatever yog's own repo-root `PROTOCOL` file reads, so it is stated a
/// second time — as a literal copied from that file — in `crate::corpus`, which
/// is what the suite's stand-in engine dials at. The two agreeing is a test
/// (`corpus::tests`) rather than a tautology, and that is the whole of the
/// defence: while the stand-in wrote its preface from *this* constant, both
/// ends of every test agreed by construction and the pin sat five versions
/// behind a live engine with the suite green (bl-e0f0).
///
/// **This line drifted seven times under the old discipline** (bl-e0f0,
/// bl-f88f, bl-dc5f, bl-605f, bl-44b4, bl-272d, bl-dc8a), four of them for an
/// optional field on a shape a foot never decodes. None of the four would move
/// it now: they are editions, and an engine an edition on is one this foot
/// still dials.
///
/// **It is not declared here.** The repo-root `PROTOCOL` file states it, one
/// line, and `build.rs` compiles that into the constant re-exported below
/// (bl-c618). Bump it by editing that line; nothing under `src` says the
/// number. It is a file because the release gates that read it are other
/// repositories FETCHING one path out of a tree they do not build, and a Rust
/// path is not a stable address for that: yog's own split of
/// `src/wire/hello.rs` left the old path re-exporting, which a build cannot
/// notice and a regex reads as no declaration.
pub use protocol::PROTOCOL;

/// **The edition this build vendored** (REMOTE §3.2): the newest stamp in its
/// copy of the engine's shape ledger, written beside [`PROTOCOL`] in the
/// preface so the peer can tell what this end can spell without a bump.
///
/// **It is 8, ten under the [`FLOOR`], and that is the honest number.** An
/// edition states what this end can READ, and a foot reads nothing stamped past
/// 8 — every addition since is on a seat-facing shape it never decodes. Its one
/// home is the vendored ledger (`corpus::ledger::edition`); this constant is
/// the second statement of it, held equal there by a test for the reason
/// [`PROTOCOL`] is. It is a plain declaration rather than a second
/// `PROTOCOL`-shaped file because no gate fetches it: an edition holds no
/// release (§3.2 — the holds fire only on a major).
pub const EDITION: u32 = 8;

/// **The edition the current major was cut at** (REMOTE §3.2), and what a peer
/// that states none is read as.
///
/// Absent means a build from before the key existed, and every such build is at
/// the floor by definition — the floor is where the major was cut. It is the
/// conservative reading in the only direction that matters: it credits a peer
/// with the least it can be spelling.
pub const FLOOR: u32 = 18;

/// The generated constant, and nothing else: `build.rs` writes it from the
/// repo-root `PROTOCOL` file on every build the file has moved.
mod protocol {
    include!(concat!(env!("OUT_DIR"), "/protocol.rs"));
}
