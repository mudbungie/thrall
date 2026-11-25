+++
title = "Push, not poll: hold one pushing invocations read open (holding on every reconnect), run every pushed job at once with no limit, ignore heartbeats, gate on server edition 32 (yog REMOTE 5.8)"
created = 1795581368
updated = 1795581368
priority = 2
root_commit = "32be9f81c8ae1d50610ed025db0de83568b6736b"
tags = ["feature"]
+++
Host half of yog's REMOTE §5.8 (in force by yog bl-f5cd; the server's hello states edition 32 or above). Implement against the yog repository's docs/REMOTE.md §5.8 — the authority; the text below is quoted from it.

The read (REMOTE §5.8.1):

    {"op": "invocations", "holding": ["inv-7", "inv-9"]}

**`holding`** is every job this host has taken whose final `complete` the
server has not yet answered (*final*: the post carrying `capture`; a refusal
of it is an answer). It may be empty. **The key's presence is the whole
signal** that this is a pushing read — there is no flag beside it. Anything
but an array of strings is refused, by name. On the line it is `/invocations
--holding <id>…`, and `--holding` alone is the empty list.

The frames a pushing read is answered with (REMOTE §5.8.1, server rules 3 and 5):

3. **It writes one reply frame per job**, `{"ok": true, "kind":
   "invocations", "rows": [<one row>]}` — the serial reply's own shape with
   exactly one row — the moment that job is queued for the certificate,
   whether or not the host is running others. Each job handed joins what the
   read leaves alone, so no read hands one slot twice.

5. **A read that has written nothing for thirty seconds writes a heartbeat**:
   exactly `{"ok": true, "kind": "invocations", "rows": []}`. It is a reply
   frame with no rows — **never the zero-length terminator** (§3), which
   would end the read — and a host ignores it. Its job is to be a write, so a
   dead connection fails one; and to be a frame the host can time.

Superseded is unchanged (REMOTE §5.1): {"ok": true, "kind": "invocations", "rows": [], "superseded": true} then the terminator — stop the channel, do not dial again.

The rule this ball implements, verbatim (REMOTE §5.8.1):

**What a host owes** — the rule every host implements, stated once:

- **Push only to a server whose hello states an edition at or above 32**
  (§3.2's discovery; `request/invocations/holding` is stamped 32 in
  `corpus/shapes.json`). Below it, the server would read `holding` as an
  unknown key and the read as serial; the host keeps today's serial loop.
- **Keep exactly one pushing read open, always**, working or not, and send
  `holding` — the full list, possibly empty — on every (re)connect.
- **Start every pushed job at once, on its own thread, with no limit**, and
  post its results on **its own connection**, never on the read's.
- **Keep an id in `holding` until its final `complete` is answered**
  (landed or refused). Then any interleaving of a result and a reconnect is
  safe: listed and finished is ignored, unlisted and finished has nothing to
  hand out.
- **A pushed id already in `holding` is not started twice.**
- **Ignore heartbeats** — a frame whose `rows` is empty and carries no
  `superseded` — and **treat four heartbeat intervals with no frame (two
  minutes, §5.1's transport bound) as a dead connection**: reconnect, with
  `holding`.
- **Never `advertise` while its own pushing read is open**: an accepted
  advertisement ends that read as superseded (§5.1), which the host would
  then obey. Advertise first, then open the read.
- **Hold no backlog**: what it has not started is the server's.

To do: keep one pushing read open always; start every pushed job at once on its own thread with no limit; post each job's result on its own connection; send holding on every (re)connect; ignore heartbeats; gate on the server's edition (>= 32), keeping today's serial loop below it; hold no backlog. Delete the re-advertisement after each hand-off and its restored notice for a pushing channel (REMOTE §5.8 makes it unnecessary: the read is never closed while working). Re-vendor the corpus ledger at edition 32.

Tests owed: a job pushed while another runs starts at once; reconnect sends the full holding list; a heartbeat frame is not treated as work or as the end; four quiet intervals reconnect; superseded stops; an edition-31 server gets the serial loop.