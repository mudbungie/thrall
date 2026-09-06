+++
title = "the protocol pin is 8 while the engine speaks 13, so a foot built from main cannot dial an engine built from main — bl-e0f0 again, one bump later"
created = 1788673497
updated = 1788673497
priority = 1
root_commit = "32be9f81c8ae1d50610ed025db0de83568b6736b"
tags = ["usability-r1"]
+++
Round 1, devadmin lane. A regression naming bl-e0f0, which was the same defect at 2 vs 8.

    thrall  src/channel/hello.rs:55  pub const PROTOCOL: u32 = 8;
    thrall  src/corpus.rs:46         pub(crate) const PROTOCOL: u32 = 13;  (was 8)
    yog     src/wire/hello.rs:118    pub const PROTOCOL: u32 = 13;

Both trees at their main tips, both built with `cargo build --release`.
`thrall run` against that engine, with a foot leaf minted by the engine`s own
`yog wire-certs` and channel material placed by hand:

    ops: wire protocol mismatch: this foot speaks version 8, the engine speaks
    13. There is no negotiation — upgrade the older component until both speak
    one version.. Dialling this engine again in 1s.
    ... again in 2s. ... in 4s. ... in 8s.

So the shipped foot cannot serve the shipped engine at all. Every scenario
this lane exists to drive was blocked on it; the lane continued only by
building thrall out of a scratch copy of the tree with the two constants set
to 13, after which the whole leg — advertise, invocations, complete — worked
against the real engine with no other change. So the frames a foot speaks have
NOT moved between 8 and 13; only the equality has.

That last fact is the shape of the fix as much as the evidence for it.
`src/corpus.rs` already carries the argument: "What a foot speaks is smaller
than the protocol, so most bumps are not its business ... It still cannot dial
across any of them, because the preface is one integer compared for equality".
bl-e0f0`s remedy was to copy the engine`s constant and to make the stand-in
engine state its own literal so a drift is a red test. That worked — the
suite is honest — but nothing MAKES the copy happen, so the pin goes stale the
next time yog bumps, which is now.

Two things to decide, and the second is the real one:
1. bump to 13 (one line, plus the corpus doc line that still says the engine
   reads 8);
2. what stops the third occurrence. Options seen from here: a released-yog
   version pin that CI reads the constant out of; a test that fails when
   thrall`s pin is behind the newest yog release on crates.io; or the ruling
   that the foot`s preface is a RANGE, since the frames demonstrably did not
   change across five bumps. The equality is defensible ("the version is the
   engine`s build") but it has now cost two balls and one blocked test lane,
   and the cost falls on the operator as a box that dials forever.

Severity p1: a shipped foot cannot connect to a shipped engine.