+++
title = "a protocol mismatch is redialled forever instead of ending the channel: the one refusal no wait can cure is the one thrall waits on"
created = 1788673922
updated = 1788673922
priority = 3
root_commit = "32be9f81c8ae1d50610ed025db0de83568b6736b"
tags = ["usability-r1"]
+++
Round 1, devadmin lane. Beside bl-f88f, which is the pin itself; this is what the loop does with it.

    ops: wire protocol mismatch: this foot speaks version 8, the engine speaks
    13. There is no negotiation — upgrade the older component until both speak
    one version.. Dialling this engine again in 1s.
    ... again in 2s.  ... in 4s.  ... in 8s.  ... in 16s ... forever.

The sentence is right and says so itself: "There is no negotiation — upgrade
the older component". Nothing about a later dial can change either integer, so
this is by construction a channel that cannot be served at all — which the
README already names as the class that ENDS:

    "Restarting the process is still this machine`s own supervision — a foot
    that cannot be a foot at all exits and says why."

`run::redial` classifies it as `Ending::Again` anyway, so it takes the backoff
alongside the endings the backoff is for (a lid, a relay switch, a dead
network — all of which a later dial genuinely fixes).

WHAT IT COSTS, in the order it bites:

- supervision sees a healthy process. There is no exit code, so systemd or a
  container restart policy has nothing to act on and no alert fires. A foot in
  this state looks alive and offers nothing, which is precisely the failure
  mode bl-916d`s redial loop was written to END, arriving through the door
  the loop opened.
- the log fills at a settled minute`s cadence with one long identical line,
  forever, on a box nobody is reading. This lane`s first sighting of the
  mismatch was a container whose logs had to be dug for.
- the engine end says nothing at all: no client row, no ops entry, no refusal
  the operator sees. The whole failure lives in the foot`s stderr.

ASKED FOR: a hello refusal is `Ending::Over` — thrall prints the sentence once
and exits non-zero, which is what "restart policy belongs to this machine`s own
supervision" means for a fault a restart cannot fix either. That is not a
general rule about refusals; the version preface is the one refusal whose
answer is a deploy, and telling it apart from a dropped TCP is what
`channel::hello` already does.

COSMETIC, same line, already noted on bl-f88f: "…one version.. Dialling" —
two full stops, from a period inside the message plus the one the wrapper
appends.