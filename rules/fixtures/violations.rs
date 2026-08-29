//! DELIBERATE ast-grep fixture — NOT part of the crate and never compiled. It
//! lives under `rules/`, outside `src/`, and is named by no Cargo target. Its
//! only job is to be flagged by every rule in `rules/`.
//!
//! Smoke test, BOTH DIRECTIONS (see the `rules-audit` Makefile target):
//!   - `ast-grep scan rules/fixtures` MUST exit non-zero, flagging every
//!     deliberate violation below:
//!       * no-rc-refcell.yml            → violations 1–2
//!       * no-pub-borrow-return.yml     → violations 3–4
//!       * no-pub-generic-bounds.yml    → violation 5
//!       * no-named-lifetimes.yml       → violation 6
//!       * no-assert-outside-tests.yml  → violation 7
//!       * no-lint-suppression.yml      → violation 8
//!   - `ast-grep scan src` MUST exit zero.
//!
//! One direction alone is worthless. A clean `src` proves nothing if a rule's
//! pattern has silently stopped matching anything at all — which is exactly
//! how a gate passes as green forever. If any violation below ever stops being
//! flagged, that rule has regressed.
//!
//! The rules that are NOT represented here are the ones thrall has nothing to
//! measure yet: `unsafe` confinement, the lock chokepoint, and the spawn
//! boundary. They land with the surfaces that need them (bl-1827), each with
//! its own fixture — never installed vacuous.

// Violation 1: an `Rc` — banned everywhere, no test carve-out.
fn uses_rc() {
    let _r: std::rc::Rc<u32> = std::rc::Rc::new(0);
}

// Violation 2: a `RefCell` — banned everywhere.
struct HoldsRefCell {
    inner: std::cell::RefCell<u32>,
}

// Violation 3: a `pub fn` returning a borrow (the `reference_type` in
// `return_type`). The elided lifetime is the hidden coupling this bans: the
// caller is now tied to the callee's storage without a signature saying so.
pub fn borrow_return(s: &str) -> &str {
    s
}

// Violation 4: a `pub fn` returning an opaque `impl Trait` (the
// `abstract_type` in `return_type`). Under edition 2024's implicit capture an
// `impl Trait` return smuggles borrows invisibly.
pub fn opaque_return() -> impl Iterator<Item = u32> {
    std::iter::empty()
}

// Violation 5: a `pub` item carrying a generic bound (the `type_parameter`
// with a `trait_bounds` child). An UNbounded `<T>` would be clear; the `: Ord`
// is what fires, because a bound on the public surface forces monomorphization
// onto every consumer.
pub struct PubBound<T: Ord> {
    pub first: T,
}

// Violation 6: a named lifetime (the `lifetime` node `'a`; the rule's `not`
// excludes only `'static` and `'_`, which name nothing). Borrow on the way in,
// elided; hand back owned on the way out — then no signature ever needs to
// name one.
pub struct Held<'a> {
    r: &'a str,
}

// Violation 7: an `assert!` outside any test (a `macro_invocation` whose
// `macro` field is `assert`, not inside a `#[cfg(test)]` mod).
fn asserts_in_prod(x: u32) {
    assert!(x > 0, "prod should never assert");
}

// Violation 8: a lint suppression outside tests (the `attribute_item` matching
// `allow(`). Policy lives in Cargo.toml `[lints]`, paired with a
// justification; prod code carries no inline `#[allow]`.
#[allow(clippy::needless_return)]
fn suppresses_a_lint() -> u32 {
    return 0;
}
