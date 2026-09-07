+++
title = "the protocol pin is 17 while yog's main carries 18: vendor the number before the engine publishes it, not after a dead unit"
created = 1788754660
updated = 1788754661
claimant = "Cantaloups-T7"
priority = 1
root_commit = "32be9f81c8ae1d50610ed025db0de83568b6736b"
tags = ["usability-r3"]
+++
The vendored pin is 17. yog's `main` carries `PROTOCOL = 18`, so the seventh
re-vendor is due — and it is due BEFORE yog publishes, not after.

## Why this is filed ahead of the failure rather than off one

DESIGN §3.6 records six previous drifts (bl-e0f0 at 2, bl-f88f at 8, bl-dc5f
at 13, bl-605f at 14, bl-44b4 at 15, bl-272d at 16), and every one was found by
a foot that could not dial — bl-272d off a `failed` unit on two boxes, because
a mismatch now exits instead of retrying. That order is the defect. yog
`docs/REMOTE.md` §3 states the rule the other way round: the consumers' mains
carry the number first, then yog publishes, then the consumers publish.

Landing the constant on `main` is held by neither gate — it is what gate 1
(yog's release hold, bl-bca2) waits for. So the seventh move is made while the
engine is still unpublished, the first time the ledger's own ordering is
honoured rather than reconstructed after a dead unit.

## What 18 is, and why a foot decodes none of it

Three lanes raised the wire in one night and one number carries all of them
(yog `src/wire/hello/version.rs`, the 17 → 18 entry). Every shape is
seat-facing:

- `reply/follow`'s tool-window entry gained `held` (yog bl-58bb) — the
  capability control's reason for parking the call.
- `reply/steps` rows gained a fourth `framing` word, `in_flight` (yog bl-ab53)
  — a new VALUE, which no shape signature can see.
- `request/answer` and `reply/answered` gained `scope` (yog bl-94a5) — how far
  one capability answer stands, required in both directions.

A foot decodes none of them, and still cannot dial across any of them, because
the preface is one integer compared for equality: the version states the
engine's *build*, never which frames this end reads.

## The work

1. Compare the seven foot-decoded shapes in `src/corpus.rs` byte for byte
   against the engine's `corpus/` — `request/{advertise,invocations,complete}`
   and `reply/{advertised,invocations,routed,refusal}`. Expected: identical,
   stamps unchanged at 2/1/1/8/2/1/1.
2. Move the constant in `src/channel/hello.rs` and `src/corpus.rs`.
3. Extend the §3.6 ledger: the pin is 18, the drift count is SEVEN, and 18's
   three shapes join the seat-facing list.

## What holds after this lands

`merge-release-pr` (bl-635b) holds this crate's release while `PROTOCOL`
exceeds the newest published engine's `v<x.y.z>` tag. 18 is unpublished, so the
release PR parks by design and clears itself when yog publishes. That hold is
the gate working, not a defect to route around.