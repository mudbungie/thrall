+++
title = "the protocol pin is 2 while the engine speaks 8, so a foot cannot dial a current engine — and the stand-in engine writes its preface from this crate's own constant, so nothing can catch it"
created = 1788399973
updated = 1788399973
priority = 1
root_commit = "32be9f81c8ae1d50610ed025db0de83568b6736b"
+++
**Filed from the engine side** while landing yog bl-66d4, which raised yog's
`PROTOCOL` to 8. Two facts, one edit — but the second is why the first went
unnoticed, so they are one ball.

## The pin is five versions behind, and that is an outage

`src/channel/hello.rs` reads `pub const PROTOCOL: u32 = 2`. yog's
`src/wire/hello.rs` read 7 before today and reads **8** now. The preface is
checked by exact equality on both ends — yog's `hello::admit` answers
`peer == Some(u64::from(PROTOCOL))` and refuses otherwise, naming both numbers.

So a current foot **cannot dial a current engine at all**. Not a degraded lane,
not a missing field: the preface is refused before any gesture is sent. The two
crates' other clients are already correct (the seat crate and the phone client
both pin 7), which is what makes this a drift in one component rather than a
protocol yog forgot to publish.

The number is not thrall's own counter. Its doc says *"The protocol this build
speaks ... The integer moves when the existing shape changes meaning: the
framing, the envelope, or what a spelling already in use is taken to say."*
That is yog's changelog paragraph, and the value must be whatever yog's is;
there is nothing for a foot to version independently.

## Why the suite cannot see it, which is the defect under the defect

The tests drive a stand-in engine written here, and that stand-in writes its
preface from **this crate's own constant**. So both ends of every test agree by
construction, at any value, including a value no engine has spoken for months.
The one thing the preface exists to catch is the one thing the harness makes
unfalsifiable — a test that cannot fail.

The fixture must state the number as a **literal** it got from yog, not from
`hello::PROTOCOL`. Then a bump is a red test naming both numbers, which is
exactly the sentence the wire would have given.

## The re-vendor itself, and the one field to consume

Raise the pin to 8 and re-vendor `corpus/` from a yog checkout at that number.
The shape that moved is the one this component cares about most:

`reply/advertised` gained **`wrote`**, a required boolean — `{"kind":
"advertised", "ok": true, "wrote": true|false}`. It is false when the engine
found the set identical and compared, true when it wrote the document. That is
bl-2d78's missing receipt: `run::present` re-asserts at the end of every
hand-off and could not tell a no-op from a restoration, so a box whose set had
been blanked by a rival healed **silently**.

Consuming it is the whole remedy: `present` reads the reply it already decodes,
and a `true` on any re-assertion after the first presentation is this machine
learning it was disarmed while it was absent. Print it. A `true` on the FIRST
presentation of a channel is ordinary and says nothing.

Cites: yog bl-66d4, yog REMOTE §5.1, bl-2d78, bl-916d.