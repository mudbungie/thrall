+++
title = "grows-only reader and the edition replay: PROTOCOL 19 as a major, catch-all words, projection + mutation over the seven vendored shapes, edition in the hello"
created = 1788926007
updated = 1788926007
priority = 1
root_commit = "32be9f81c8ae1d50610ed025db0de83568b6736b"
tags = ["usability-r3"]
+++
Child of yog bl-e598 (the design; REMOTE 3.2 is the authority). thrall today: unknown keys ignored structurally (`src/json.rs`); `bool_of`/`str_of` required with no default; the only wire word decoded is the reply `kind`, which stays strict; the seven shapes are vendored as string consts in `src/corpus.rs` with stamps in prose only; no `shapes.json`, nothing reads a stamp.

Thrall-specific: vendor yog's `corpus/shapes.json` stamps for the shapes it speaks (as a `.rs` const beside the frames, so nothing ships) and run the projection and mutation replays over its four reply shapes (`advertised`, `queued`, `routed`, `refusal`); `wrote` (stamp 8, below the floor) stays required. `Failure::Skew` and the `unusable kind` refusal stay as they are. Document rule 2(b) in `src/json.rs` for the next field. DESIGN 3.6's by-hand re-vendor note becomes the replay.

## The contract (yog REMOTE 3.2, bl-e598 — read it there; this is the consumer's half)

yog's `PROTOCOL` is now a MAJOR: it moved 18 -> 19 and thereafter moves only on a breaking change (a field removed or re-typed, a meaning changed under a spelling in use, a field the engine newly requires on a request). Every addition — a field, a word, an op, a reply kind — ships with no bump and is stamped an EDITION per field path in yog's `corpus/shapes.json` (`signature` is now `{ "<path>:<type>": <edition> }`; the file also carries `floor` — the edition the major was cut at, 18 — and `deprecated`). The hello stays strict equality on `PROTOCOL`, fail-closed, no negotiation; the engine additionally states `edition` in its hello (yog child ball).

This client owes, in ONE ball:

1. **`PROTOCOL` 18 -> 19** at the repo root. The release gate holds this repo's release until yog publishes 19, and yog's hold keeps 19 unpublished until all three consumer mains carry it — so the bump lands here with the reader change, never before it.
2. **Grows-only reader.** (a) unknown key ignored — already structural, keep it so; (b) a key stamped above `floor` reads as its default when absent, and the default is the fact before the field existed — no such key exists today, so this is the rule for every future field, written where the field readers are documented; (c) an unknown word in any vocabulary maps to a named catch-all carrying the word, rendered honestly ("unknown state: <word>"), never to the nearest known word and never to a refusal of the row; (d) the reply `kind` (and, where decoded, the request `op`) stays strict by name.
3. **Hello.** Write `edition` beside `protocol` (this build's vendored corpus edition = the newest stamp in its `shapes.json`); read the engine's, absent = `floor`. Keep the engine's edition where a control can ask `spells(shape, key) = stamp <= engine edition` and grey what the engine cannot spell; render an absent post-floor field as *this engine cannot say* rather than as the default, when the edition says so.
4. **The conformance test**, beside the existing corpus replay: (i) projection — for every reply shape this client decodes and every edition e in floor..=edition, delete from each frame every key whose stamp > e, decode, assert nothing refuses; (ii) word mutation — for every string-typed path in those shapes other than `kind`, replace the value in one frame with a token no build has heard of, decode, assert nothing refuses. Both read the stamps out of the vendored `shapes.json`; re-vendor it (new format) first.
5. **The parity ledger records what this client does not RENDER, never what it refuses.** A valid frame of a kind this client never asks for is a parity fact with a reason, not an unreadability.