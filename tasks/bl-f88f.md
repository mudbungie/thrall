+++
title = "the protocol pin is 8 while the engine speaks 13, so no foot can dial a current engine — bl-e0f0 has recurred"
created = 1788673284
updated = 1788673505
priority = 1
root_commit = "32be9f81c8ae1d50610ed025db0de83568b6736b"
tags = ["usability-r1"]
+++
Round-1 swdev lane, scenario step: enrol a local foot so agents can execute (yog docs/STORIES.md S3/S11).

Gesture, on main tips of both repos (thrall 0.0.5, yog 0.0.38, both freshly `cargo build --release` from their main tips):

    thrall run     # data root provisioned by hand: tools.json + wire/workspaces/<ws>/{ca,client}.pem, client.key, address

Verbatim, on stderr, repeating with backoff:

    <ws>: wire protocol mismatch: this foot speaks version 8, the engine speaks 13. There is no negotiation — upgrade the older component until both speak one version.. Dialling this engine again in 1s.

Expected: the channel comes up and the foot advertises its set.

This is bl-e0f0 recurring exactly — that ball reads 'the protocol pin is 2 while the engine speaks 8'. The pin is `channel::hello::PROTOCOL` (`src/corpus.rs` restates it as a copied line, and its own prose already says 'PROTOCOL sat five behind'). yog's is `src/wire/hello.rs:118: pub const PROTOCOL: u32 = 13`.

Severity p1: with no negotiation, a foot at any other value cannot dial at all, so `thrall` on main is unusable against `yog` on main. Every scenario that needs execution on a named box is blocked; the only working execution path today is yog's own engine-side built-ins (REMOTE §5.4 rung 3), which is the fallback the lane was designed to be better than.

Second, smaller defect in the same line: the sentence ends '…one version..' — two full stops, from a period inside the message plus the one the wrapper appends.

Standing question this raises rather than answers: bl-e0f0 fixed the value and this is the second time the value has drifted, so the defect is that the pin is a hand-copied constant with nothing that fails when the two ends disagree. A pin that drifts twice wants a mechanism, not a third bump.

---

devadmin lane, round 1: confirmed independently, with one extra datum the fix wants. The lane got past it by building thrall from a scratch copy of the tree with both constants — channel::hello::PROTOCOL and corpus::PROTOCOL — set to 13, changing nothing else. Against the real yog 0.0.38 engine that foot then completed the whole leg: advertise, invocations, complete, five tools, dozens of captures over a container channel, plus the drop/redial and two-feet cases. So the frames a foot speaks did NOT move between 8 and 13; only the equality did. That is evidence for the ruling this ball already asks for — the preface being one integer compared for equality is what turns five seat-facing bumps into a foot outage.
