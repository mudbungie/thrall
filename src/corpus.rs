//! **The engine's conformance corpus, vendored as literal text** (yog's
//! `docs/REMOTE.md` §3) — and the one place in this crate that states what the
//! far end says rather than what this crate believes.
//!
//! REMOTE §3 publishes `corpus/request/<op>.json` and `corpus/reply/<kind>.json`,
//! one file per shape holding that shape's frames verbatim, and states what a
//! client owes them: *"decode every frame in both directories into its own
//! types, and round-trip what it emits — decode then re-encode must return the
//! frame exactly"*, with the concession that *"the foot's surface is small
//! enough that it may consume only the subset of shapes it speaks"*. This
//! module is that subset, and [`tests`] is the owing paid.
//!
//! **The text is copied, not derived, and that is the whole mechanism**
//! (bl-e0f0). A conformance fixture built out of the crate it is testing agrees
//! with itself at any value — which is exactly how [`PROTOCOL`] sat five
//! versions behind a live engine while every test passed: the stand-in engine
//! wrote its preface from `channel::hello::PROTOCOL`, so both ends of the wire
//! were one constant wearing two names. Everything here is a literal got from
//! the engine, so a drift is a red test naming both numbers instead of a
//! channel no operator can open.
//!
//! **It is `.rs` rather than vendored `.json`, and that is deliberate.** The
//! files would have to be ruled into `Cargo.toml`'s `include` allowlist to
//! survive a build from the registry, and would then *ship* in the crate — a
//! test fixture in a released binary's tarball. Literals under `src` are
//! already `cfg(test)`, cost a released build nothing, and are read by the same
//! decoders at the same strictness.
//!
//! **Three things are vendored, not one** (§3.2, bl-6fcf). The frames below;
//! [`ledger`], the edition every field path appeared at, which is what turns
//! *"a field was added"* from prose into an input; and the number, which is now
//! a MAJOR and moves only on a break. [`replay`] is what spends the second on
//! the first: one record projected to every edition an engine can be at, and
//! read under a word no build has heard of.

/// The edition stamps: what appeared when, per field path.
pub(crate) mod ledger;
/// The projection and word-mutation replays REMOTE §3.2 asks a consumer for.
mod replay;

/// **The protocol number, got from the engine, and it is a MAJOR** (REMOTE
/// §3.2). yog's repo-root `PROTOCOL` file reads `19`, and this is that number
/// copied.
///
/// It is what the suite's stand-in engine states, so every channel test dials
/// across the same equality a real one does — and
/// [`channel::hello::PROTOCOL`](crate::channel::hello::PROTOCOL) agreeing with
/// it is a test rather than a tautology.
///
/// **What moves it now.** 18 → 19 was the last bump of the old kind and the
/// first of the new: from here the integer moves only on a BREAK — a field
/// removed or re-typed, a meaning changed under a spelling still in use, a
/// field the engine newly requires on a request — and every addition ships
/// unbumped, stamped an edition in [`ledger`] instead. So the seven re-vendors
/// this line paid for between 2 and 18 (bl-e0f0, bl-f88f, bl-dc5f, bl-605f,
/// bl-44b4, bl-272d, bl-dc8a), five of them inside three sessions and four of
/// them for a field on a shape a foot never decodes, are the class §3.2 closed:
/// none of the four would move this number today.
///
/// **What it still cannot catch, and what now stands beside it.** Both
/// constants live in this tree, so what this pin defends is a re-vendor that
/// moved the frames and forgot the number; an engine that moved while this
/// crate stood still is invisible to a fixture, and only a real dial meets that
/// skew. What changed is that the dial no longer has to: an engine one edition
/// on is an engine this foot still speaks to, because the equality is on the
/// major and the additions are what the replay already proved this crate reads
/// through.
pub(crate) const PROTOCOL: u32 = 19;

/// `corpus/request/advertise.json`, edition 2 in [`ledger`] — the empty set, one
/// ordinary element, and the element carrying §5.1's optional fourth fact.
pub(crate) const ADVERTISE: [&str; 3] = [
    r#"{"op":"advertise","tools":[]}"#,
    r#"{"op":"advertise","tools":[{"description":"run a command","input_schema":{"properties":{"command":{"minLength":1,"type":"string"}},"required":["command"],"type":"object"},"name":"Bash"}]}"#,
    r#"{"op":"advertise","tools":[{"description":"run a command","input_schema":{"properties":{"command":{"minLength":1,"type":"string"}},"required":["command"],"type":"object"},"name":"Bash","subject_cwd":true}]}"#,
];

/// `corpus/request/invocations.json`, edition 1. The follow-class read
/// carries no field at all — a connection drains its own queue.
pub(crate) const INVOCATIONS: &str = r#"{"op":"invocations"}"#;

/// `corpus/request/complete.json`, edition 1.
pub(crate) const COMPLETE: &str = r#"{"capture":{"exit_code":3,"stderr":"warned\n","stdout":"hello\n"},"invocation":"inv-1","op":"complete"}"#;

/// `corpus/reply/advertised.json`, **edition 8** — the newest stamp this foot
/// vendors, and the bump this module was written for. `wrote` is required
/// because 8 is under the floor, and `false` is the ordinary re-presentation.
pub(crate) const ADVERTISED: [&str; 2] = [
    r#"{"kind":"advertised","ok":true,"wrote":false}"#,
    r#"{"kind":"advertised","ok":true,"wrote":true}"#,
];

/// `corpus/reply/invocations.json`, edition 2 — nothing queued, one
/// ordinary row, and one carrying the worktree lane's `cwd`.
pub(crate) const QUEUED: [&str; 3] = [
    r#"{"kind":"invocations","ok":true,"rows":[]}"#,
    r#"{"kind":"invocations","ok":true,"rows":[{"input":{"command":"ls -l","timeout":30},"invocation":"inv-1","tool":"Bash"}]}"#,
    r#"{"kind":"invocations","ok":true,"rows":[{"cwd":"/w/home/agents/c-1","input":{"command":"printf made > out.txt"},"invocation":"inv-2","tool":"bash"}]}"#,
];

/// `corpus/reply/routed.json`, edition 1 — the slot as it stands, with
/// the capture absent while the work is still out.
pub(crate) const ROUTED: [&str; 2] = [
    r#"{"invocation":"inv-1","kind":"routed","ok":true}"#,
    r#"{"capture":{"exit_code":3,"stderr":"warned\n","stdout":"hello\n"},"invocation":"inv-2","kind":"routed","ok":true}"#,
];

/// `corpus/reply/refusal.json`, edition 1 — the envelope with no
/// `kind`, which is the one shape a refusal may wear.
pub(crate) const REFUSAL: &str = r#"{"error":"unknown op \"fhtagn\"","ok":false}"#;

mod tests;
