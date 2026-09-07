+++
title = "the protocol pin is 13 while yog main speaks 16, so no foot can dial a current engine: bl-e0f0, bl-f88f and bl-dc5f have all recurred, and nothing gates a yog PROTOCOL bump on its two consumers"
created = 1788745990
updated = 1788745990
priority = 1
root_commit = "32be9f81c8ae1d50610ed025db0de83568b6736b"
tags = ["usability-r2"]
+++
Round 2, lane swdev.

## The state

    thrall  0.0.11 installed, tree 034f6db: src/channel/hello.rs `pub const PROTOCOL: u32 = 13`
    lernie  0.1.34 installed, tree 1763302: src/channel/hello.rs `pub const PROTOCOL: u32 = 13`
    yog     0.0.49 installed: 15;  main 5cd95986: `pub const PROTOCOL: u32 = 16`

The released seat cannot dial the released engine either:

    lernie workspaces
    (this box own engine)
        wire protocol mismatch: this seat speaks version 13, the engine
        speaks 15. There is no negotiation — upgrade the older component
        until both speak one version.

and a foot on 13 against a main-tip engine on 16 is the same refusal. The lane
could only proceed by copying the thrall and lernie trees aside, raising the
one constant to 16 in each and `cargo build --release` — round 1s workaround
for bl-f88f, performed again unchanged.

## The recurrence, which is the actual ball

    bl-e0f0  pin 2,  engine 8   p1  closed
    bl-f88f  pin 8,  engine 13  p1  closed — "bl-e0f0 has recurred"
    bl-dc5f  pin 13, engine 14  p1  closed — "bl-f88f went stale within the hour"
    this     pin 13, engine 16

yog went 13→14 (bl-fec6), 14→15 (bl-5305) and 15→16 (bl-e59e) during the
round-1 fix wave. Each bump is lawful and each left the foot dead, and nothing
on either side fails when it happens: yogs suite is green at 16, thralls is
green at 13, and the mismatch is only discoverable by running the two together.
Bumping the constant a fourth time fixes this week.

The consumer side of the same fact is lernie bl-183b (re-vendor the wire
corpus), still `ready` at p2 while the seat is three versions behind — a
released seat that cannot reach a released engine is not a p2.

## What would end it

Not a bigger number. Something that makes a yog PROTOCOL bump fail until its
two consumers move: the corpus thrall and lernie vendor is generated from
yogs, so the pin is derivable rather than copied, and a bump that has not
been re-vendored is a red build somewhere rather than a dead dial on an
operators box. `src/corpus.rs` already carries the line as a comment quoting
yogs — the comment is the invariant, unenforced.

## Severity

p1: the enrol-a-foot story, and therefore every story where an agent executes
anything, is dead at the first command for anyone using released binaries.