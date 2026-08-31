+++
title = "the deadline cascade signals the child, not its process group: a tool that starts something outlives its own deadline"
created = 1788061013
updated = 1788138439
claimant = "OrderFerrier"
priority = 3
root_commit = "32be9f81c8ae1d50610ed025db0de83568b6736b"
+++
The executor's deadline sends `SIGTERM` to the child it spawned and then
`SIGKILL`s it (`src/sys.rs`, `src/exec.rs`). Anything the *child* started is
untouched: it is reparented and keeps running, past the deadline, past the
capture, and past the foot's own exit.

That is stated plainly in DESIGN §3.5 rather than papered over, which is the
containment-honesty clause working — but it is still a gap between what a
deadline means and what it does. A tool that forks a helper and returns leaves
that helper on the box, and the far end reads a capture that says the tool was
terminated.

**The shape of the fix.** Put the child in a process group of its own at spawn
(`CommandExt::process_group(0)`, safe std on Unix, so it belongs in
`src/spawn.rs` beside the environment scrub) and send the signal to the group by
negating the id (`kill(-pgid, …)`, which is the one thing `src/sys.rs` currently
refuses on purpose — a non-positive argument means a group, and today that is a
hazard rather than an intent).

**What has to be answered before it lands.**

- The `pid > 0` guard in `sys::terminate` exists so a widened signal cannot
  happen by accident. Sending to a group means either a second function that
  says so in its name, or one that takes a group as its own type. A boolean flag
  would be the worst of the three.
- `process_group` is Unix-only, and the crate carries no `cfg` axis today.
  Adding one is a real cost, and the alternative — declaring `setpgid` beside
  `kill` in the confined file — trades it for one more raw call.
- Whether the grace applies to the group or to the child. A helper that ignores
  `SIGTERM` should not extend the foot's wait past the deadline it already
  overran.

Not urgent: a foot's tools are the operator's own executables, and an operator
who wants a helper to outlive a call can say so. It is filed because the gap is
real and the doc's sentence should have a ball behind it.