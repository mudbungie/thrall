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
//! **What a foot speaks is smaller than the protocol, so most bumps are not
//! its business.** Between PROTOCOL 2 and 8 exactly one shape a foot decodes
//! moved: `reply/advertised` gained `wrote` at 8. The five bumps between are
//! seat-facing — a conversation row's `failure` (3), the queue row's `flag`
//! (4), `reply/governing`'s lineage keys (5), `reply/providers`' `effort` and
//! `priority` (6), `reply/help`'s `surface` (7) — and a foot decodes none of
//! them. It still cannot dial across any of them, because the preface is one
//! integer compared for equality: the version is the *engine's build*, not a
//! statement about the frames this end happens to read.

/// **The protocol number, got from the engine.** yog's `src/wire/hello.rs`
/// reads `pub const PROTOCOL: u32 = 8`, and this is that line copied.
///
/// It is what the suite's stand-in engine states, so every channel test dials
/// across the same equality a real one does — and
/// [`channel::hello::PROTOCOL`](crate::channel::hello::PROTOCOL) agreeing with
/// it is a test rather than a tautology.
pub(crate) const PROTOCOL: u32 = 8;

/// `corpus/request/advertise.json`, stamped PROTOCOL 2 — the empty set, one
/// ordinary element, and the element carrying §5.1's optional fourth fact.
pub(crate) const ADVERTISE: [&str; 3] = [
    r#"{"op":"advertise","tools":[]}"#,
    r#"{"op":"advertise","tools":[{"description":"run a command","input_schema":{"properties":{"command":{"minLength":1,"type":"string"}},"required":["command"],"type":"object"},"name":"Bash"}]}"#,
    r#"{"op":"advertise","tools":[{"description":"run a command","input_schema":{"properties":{"command":{"minLength":1,"type":"string"}},"required":["command"],"type":"object"},"name":"Bash","subject_cwd":true}]}"#,
];

/// `corpus/request/invocations.json`, stamped PROTOCOL 1. The follow-class read
/// carries no field at all — a connection drains its own queue.
pub(crate) const INVOCATIONS: &str = r#"{"op":"invocations"}"#;

/// `corpus/request/complete.json`, stamped PROTOCOL 1.
pub(crate) const COMPLETE: &str = r#"{"capture":{"exit_code":3,"stderr":"warned\n","stdout":"hello\n"},"invocation":"inv-1","op":"complete"}"#;

/// `corpus/reply/advertised.json`, stamped **PROTOCOL 8** — the bump this
/// module was written for. `wrote` is required, and `false` is the ordinary
/// re-presentation.
pub(crate) const ADVERTISED: [&str; 2] = [
    r#"{"kind":"advertised","ok":true,"wrote":false}"#,
    r#"{"kind":"advertised","ok":true,"wrote":true}"#,
];

/// `corpus/reply/invocations.json`, stamped PROTOCOL 2 — nothing queued, one
/// ordinary row, and one carrying the worktree lane's `cwd`.
pub(crate) const QUEUED: [&str; 3] = [
    r#"{"kind":"invocations","ok":true,"rows":[]}"#,
    r#"{"kind":"invocations","ok":true,"rows":[{"input":{"command":"ls -l","timeout":30},"invocation":"inv-1","tool":"Bash"}]}"#,
    r#"{"kind":"invocations","ok":true,"rows":[{"cwd":"/w/home/agents/c-1","input":{"command":"printf made > out.txt"},"invocation":"inv-2","tool":"bash"}]}"#,
];

/// `corpus/reply/routed.json`, stamped PROTOCOL 1 — the slot as it stands, with
/// the capture absent while the work is still out.
pub(crate) const ROUTED: [&str; 2] = [
    r#"{"invocation":"inv-1","kind":"routed","ok":true}"#,
    r#"{"capture":{"exit_code":3,"stderr":"warned\n","stdout":"hello\n"},"invocation":"inv-2","kind":"routed","ok":true}"#,
];

/// `corpus/reply/refusal.json`, stamped PROTOCOL 1 — the envelope with no
/// `kind`, which is the one shape a refusal may wear.
pub(crate) const REFUSAL: &str = r#"{"error":"unknown op \"fhtagn\"","ok":false}"#;

mod tests;
