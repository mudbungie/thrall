# thrall — DESIGN

**Status: thrall is a working foot.** `thrall run` reads this box's tool
document, opens every channel the operator provisioned, presents what the box
offers, waits on its mailbox, runs what comes back and posts each capture — over
a real mTLS channel with a real version preface and a real child process. The
four balls that built it: the transport and the foot-grade check (bl-a4a5), the
operator's document and the advertisement derived from it (bl-05fe), the three
gestures and the loop that spends them (bl-a2ea), the executor and the capture
(bl-4cda).

What remains is deliberately absent: §2's list of what thrall may never become
and §5.1's MCP bridge. The publication question is no longer among them —
bl-006e adjudicated both of its halves, the repository publishes and 0.0.1 is
on the registry. This document states what thrall is, what it
may never become, and which invariants it inherits rather than owns. It is a
living document: amend it when reality diverges, and never code around a stale
section.

---

## 0. The two authorities, and which owns what

- **yog's `docs/REMOTE.md` is the PROTOCOL authority.** It defines the wire —
  the nouns, the boundary verbs, the identity model, the routing path. It is
  versioned, and all four components of the split implement against it. thrall
  does not get a vote on the protocol and does not restate it: this document
  cites REMOTE by section and says what thrall does about it.
- **This document is thrall's ARCHITECTURE authority** — its module map, its
  local decisions, its own invariants. It governs this component and nothing
  else.

Where thrall's code and REMOTE disagree, one of them is a bug. Where thrall's
code and this document disagree, one of them is a bug. In neither case invent a
third answer; fix the one that is wrong, and if it is the document, fix the
document.

Code style is the house **contained Rust** standard, adopted from birth. The
machine-enforced half lives in `rules/`, `Cargo.toml [lints]`, `clippy.toml`
and `deny.toml`; `README.md` summarises it. The two hard numbers are 300 lines
per source file and a 100% coverage floor.

---

## 1. What thrall is

thrall is the **foot** of the four-component split (REMOTE §12, adopted
2026-08-28): REMOTE §2's *tool host*, severed into its own separately
installable component.

REMOTE §2's nouns, unchanged and not re-minted here:

| Noun | Definition |
|---|---|
| **client** | A machine holding an operator-issued certificate. One certificate = one client identity, its leaf name. A client is a fact about a machine, not a person. |
| **tool host** | A client advertising tools into the workspaces it is registered in. |
| **registration** | The durable fact that client C participates in workspace W, on the server that hosts W. The server's half. |
| **entry** | A workspace held from the box that participates in it: the channel facts that reach it — the host engine's anchors, this box's leaf and key for it, the host's address, the name the workspace bears here. The client's half. Possession, where registration is permission. |

A thrall is a client that is a tool host and **only** a tool host. Its whole
life is one loop:

    advertise → { wait on the mailbox → execute → post the capture
                  → advertise } → forever

The trailing re-assertion is §3.7's, and it is why the loop is written with two
advertisements rather than one.

## 2. What thrall may never become

These are the boundaries of the component. Each is a thing that would be easy
to add and would dissolve the reason thrall exists.

- **It never listens.** No port, no socket, no callback address. thrall dials.
- **It never speaks first, and neither does the engine to it.** Every leg of
  the path is a reply to something the foot asked for (REMOTE §3, §5).
- **It never asks and never acts.** No query about a conversation, no gesture
  that changes one. Its certificate is foot-grade and would be refused.
- **It holds no world.** No conversations, no tasks, no transcript. It has an
  entry per workspace and a config saying what this box offers, and that is the
  whole of its durable state.
- **It never restarts its own process.** A foot that cannot be a foot at all —
  no tool document, no channel, a leaf that is not foot-grade, an engine
  refusing the set this box offers — is an exit naming the failure. Restart
  policy belongs to the supervision the operator's machine already has, and
  inventing one here would be thrall deciding how a box it does not administer
  runs a program.

  **A dropped CHANNEL is not that, and it is dialled again** (§3.8, bl-916d).
  This bullet once covered both, and the conclusion did not follow from the
  premise: supervision restarts a *process*, and a channel dropping does not
  kill one. What is left above is the half that was always right.

  **Which makes the sentence the product, and it is written for the operator**
  (bl-52ba). A supervisor's log, or a terminal, is the whole of what an operator
  gets on this path, so every refusal here names the address that failed — the
  same shape the connect refusals already had, held for the one an engine going
  away actually produces. **What this box will do next is said too, and since
  bl-916d it is said by the redial rather than by the channel**: a file that
  dials per ask and holds nothing cannot know whether anything is going to
  happen next, so `channel::failed` names the leg and the address and
  `run::redial` adds the wait in the same breath. A library's own
  diagnosis answers neither question: it names no address, and "peer closed
  connection without sending TLS close_notify", with a crate documentation URL
  after it, is a fact about TLS rather than about what to do. **It follows
  thrall's sentence and never replaces it** — right text for the reader who
  wants it, wrong text for the reader who has to act. The same rule governs the
  refusals about this box's own material: `channel::leaf` refusing bytes that
  are not a certificate says what the file should have held and cites
  `material::REMEDY`, rather than passing on the PEM reader's *"no items
  found"*.
- **It runs one invocation at a time, per channel.** Serial, which is what
  makes a busy foot *absent* at the far end — and why presence is not the
  routing predicate there. **Per channel** is the whole of the qualification and
  it is not a loosening: a box holding two entries is two trust relationships
  with two engines, and queueing one behind the other would make this foot
  absent from one of them for reasons that engine's operator cannot see. Each
  channel is a thread with its own connection, its own identity and its own
  serial loop; they share no state (`run::fan`).

## 3. Inherited invariants

Each of these is REMOTE's, not thrall's. thrall's obligation is to be
implementable only in the way that honours them.

### 3.1 Dial-in only — the engine never speaks first

REMOTE §5's invocation path, verbatim in its own words:

> the agent's tool call hits lernie's tool seam → server-side adjudication
> (`yog tool-control`, unchanged, fails closed) → the driver queues the
> invocation in that client's engine-side mailbox → the tool host, waiting on
> its follow-class read, is handed it → the client executes locally → the
> client posts the capture back as an ordinary act → the driver's poll collects
> it. **Nothing in that path is the engine speaking first.**

The consequence for thrall: the mailbox read is a *read the foot issues*. The
engine parks it and answers when there is work. There is no inbound direction
to secure, because there is no inbound direction.

### 3.2 Foot-grade certificate

REMOTE §2 says "every certificate is operator-grade within its registrations".
The split narrows that **for this class**: a thrall's leaf may **advertise and
execute only** — no ask, no act.

This closes REMOTE §9.6's stated residual (a registered client reads the trail
of every workspace it is in) for feet, without reopening §11's rejection of
per-tool and per-verb access lists: **the grade is binary, not a policy layer.**
It is one bit on a certificate, not the first row of a table that grows.

Enforcement is the server's — it is the party that can be trusted to enforce
it. thrall's obligations are the two it can keep: carry a leaf of that grade,
and refuse to be configured with anything else.

**As built (bl-a4a5), thrall fails closed where the engine fails open, and the
asymmetry is deliberate.** yog reads a subject with no `OU=foot` as operator
grade — *default-operator*, so a certificate minted before the grade existed
keeps working. `channel::leaf::foot` refuses that same certificate, and refuses
bytes that are not a certificate at all: an operator-grade leaf on a foot is a
machine holding the whole boundary in order to run commands, which is the thing
being a foot exists to give up. Neither end is defaulting; they are answering
different questions. The check runs when the channel **opens**, before anything
is dialled, so a mis-provisioned box learns it from its own configuration and
never from a connection failure.

### 3.3 Certificates arrive out of channel

REMOTE §1.4, and it does not expire: the certificate and key are carried to
this box by hand. thrall mints nothing, enrols nothing in band, and has no
bootstrap flow. A foot that could provision itself over the wire would be a
foot any wire could provision.

Key material lives **beside** any generated state, never inside it, for one
reason: a reseed must not be a revocation.

### 3.4 Local config gates what is enabled

REMOTE §5.2's tool-host config is thrall's one operator-authored document. It
is out of world, because it describes *this machine*.

The first three keys of each entry **are** the advertised element; `command`
and the optional `cwd` are the local half. The advertisement is the projection
that drops the local half — **one document, two readings** — so what a foot
offers and what it can actually run cannot drift. That is the entire reason the
config is not a second list beside the advertisement.

A tool absent from that document is a tool this box does not have. Server-side
adjudication is unchanged, stacks on top, and fails closed.

**As built (bl-05fe), an absent document is a refusal and not the empty set.**
A foot with nothing to offer has nothing to do, and starting one is an explicit
act that deserves an explicit answer — the same posture absent channel material
takes. An *empty* document is different and is honoured: `[]` is a box stating
that it enables no tool, which is a statement rather than a mistake and is the
shape a box takes while an operator is switching everything off. The set must
also be **addressable**: a name that is not a single path component, or one name
on two entries, refuses the whole file at the read rather than at the first
invocation.

**A `cwd` must be absolute, and that is part of being addressable** (bl-3c93).
A relative one resolves against whatever directory the supervisor happened to
start this process in — a place nobody wrote down, which changes when the unit
file does, and which nothing in the running system reports. It is the refusal
`paths::root` already makes about the data root (*"a relative fallback would
put an operator's certificates wherever the supervisor happened to start the
process, which is a place nobody chose and nobody can find again"*), held for
the operator's other document. **Shape is refused at the read; existence is
not.** Whether a path is absolute is a property of the file, static and knowable
when it is read, so it joins the empty argv and the duplicate name there.
Whether the directory exists is a property of the box, it can change between
the read and the run, and it is answered by the spawn — in band, naming the
directory (§3.5).

`command` is an **argv, spawned directly**. No shell, and no interpolation of
the invocation's input into it: a shell would make the declared `input_schema`
advisory and turn an operator's config file into a command-injection surface
for anything the model can type.

**`subject_cwd` is the worktree lane's per-tool consent** (REMOTE §5.4 as
amended by yog bl-77be; thrall bl-36f7). An entry carrying
`"subject_cwd": true` states that this box will execute that tool at a working
directory the *invocation* names — the conversation's own resolved cwd, which
REMOTE §5 calls the subject's location. It is part of the ADVERTISED element
(the engine routes the lane on it), so the projection carries it; absent reads
false, and a mistyped value refuses the file at the read, like every other
field here. The consent is meaningful only on a box that actually holds the
engine's worktrees — REMOTE §5.4's co-located thrall, the normal install — and
severable in the way this whole document is: deleting the key deletes the
capability.

### 3.5 Containment honesty

REMOTE §5, and this one is a design constraint on the *prose* as much as the
code:

> execution happens on a machine the adjudicator cannot inspect. Adjudication
> judges the invocation exactly as today; any containment beyond that is
> whatever the client enforces locally, and the design must not claim
> otherwise.

So: whatever containment thrall does enforce, it states plainly and does not
overstate. A sandbox thrall provides is thrall's, described as thrall's. Where
it provides none, it says none. A foot that implied more isolation than it has
would be worse than one that offered none, because the far end would act on the
implication.

**As built (bl-4cda), here is the whole of it.** thrall enforces exactly four
things locally, and it is not a sandbox:

- **The name**, from the operator's document: a tool absent from it cannot be
  invoked, and what runs is that entry's argv, spawned directly — no shell, and
  no interpolation of the invocation's input into it.
- **The directory**: the operator's `cwd` when the entry names one — or the
  invocation's own, when and only when the operator marked the entry
  `subject_cwd` (§3.4, bl-36f7). An invocation carrying a directory against an
  unconsenting entry is refused in band naming the key; a consented directory
  this box does not hold is refused in band too, because the far end named
  this box as holding it and it does not. **Either one must be absolute**, and
  a relative one is refused rather than resolved (bl-3c93) — the entry's at the
  read (§3.4), the invocation's in band, because a relative path that happened
  to exist beside this process would run the tool somewhere nobody named.
  **And a directory that is not one is reported as itself**: a fork fails for
  the program, for the argv or for the directory and the operating system
  spells all three `ENOENT`, so the sentence built from the program alone named
  the wrong party two times in three — an operator read *"`<the command>`: No
  such file or directory"* about a command that was exactly right. The
  directory is looked at after the failure, where the look is free and cannot
  refuse a fork that would have worked.
- **The deadline, over a process group**: the child is spawned as the leader of
  a group of its own, that group is asked to stop with `SIGTERM`, the grace is
  waited out on the child, and then the group is killed.
- **The size of the answer** (bl-6028): each stream of a capture is carried up
  to `exec::CAPTURE_LIMIT` and no further. It is a bound on what this box will
  *answer with*, never on what the tool may write — the tool runs to its own
  end and its exit code is its own.

Everything else is the box's. The child runs as this process's user, with this
process's environment less the git scrub the spawn boundary performs — no
namespace, no rlimit, no filesystem restriction.

**The cascade signals the group, and that is now what it means** (bl-a78e). The
group is decided at the spawn boundary rather than at the deadline — a child is
BORN leading one (`CommandExt::process_group`, safe `std`), so `Child::id` is
the group id and nothing has to be carried to the cascade for it to aim. Three
things about it are the design and not the implementation:

- **The grace is the child's alone.** The wait polls the tool this box was
  asked to run; a helper that ignores `SIGTERM` cannot extend a deadline the
  invocation has already overrun.
- **The insist is the group's either way.** The `SIGKILL` goes to the group
  whether or not the child answered the ask, because a leader with good manners
  is not a reason to leave its stragglers running — which was the whole gap.
- **`kill(2)` keeps its guard.** A group signal is a negative argument, and the
  confined file (§4, `src/sys.rs`) still refuses a non-positive one from a
  caller; the negation is its own act, inside two functions whose names say
  `group`. A signal cannot be widened by passing a different number, only by
  calling a differently-named function.

**Two things the group does not reach, stated because they are the claim's
edge.** A descendant that leaves the group under its own hand — `setsid`, or its
own `setpgid` — is outside the signal, and nothing a foot can do short of a
namespace changes that. And a tool that finishes *within* its deadline is not
signalled at all: the cascade is the deadline's, so whatever such a tool
deliberately left running is left running, exactly as a shell would leave it.
The deadline means the invocation is over; it does not mean the box is swept.

**The deadline bounds the CAPTURE, and those were two different things**
(bl-6c14). A pipe outlives the process that was given it: a read ends when
*every* writer has closed, and a tool that backgrounds a helper hands that
helper the same stdout and stderr write ends. So a foot that bounded the child
and then drained the pipes bounded nothing — the drain waited on a stranger it
never started, the invocation earned no capture, and the serial loop behind it
waited too, which is the hang §1's whole leg exists to exclude. Nothing
recovered it, and §3.8's redial would not have: the wedge was inside the
hand-off, where the channel is neither failing nor waiting on an engine — a
thread stuck in an executor never reaches an ending for anything to act on.

The half that had to give is the *waiting*, not the reading:

- **The drain asks what is there, rather than waiting for the end.** Every
  descriptor is non-blocking (`src/exec/pipes.rs`, and `fcntl(2)` beside
  `kill(2)` in §4's confined file), so a read answers with what the pipe holds
  now. Once the tool has exited, everything it wrote is already in the pipe —
  `write(2)` completed before `exit(2)` did — so one such pass is exactly what
  the capture is owed. **Bytes the tool produced are still delivered**, which
  is the property a merely-bounded drain would have thrown away.
- **A write end a stranger holds cannot extend the invocation.** The last pass
  keeps reading only while bytes keep arriving, under a wall of the cascade's
  own grace, so a helper writing as fast as the foot can read is bounded like
  everything else here.
- **One loop, and no threads.** The two drains and the input feed used to be
  threads *because* each of them could block. None of them can now, so the poll
  that already watched the child pumps all three, and the whole capture is
  inside the one clock. It also removes the pipe-buffer hazard as a separate
  concern rather than as a separate mechanism: the thing that watches the
  deadline is the thing that reads, so a tool writing more than a pipe holds is
  drained by the same tick that would have timed it out.

**And the capture is bounded in SIZE by the same reasoning** (bl-6028). The
wire carries one JSON frame per gesture and refuses one past
`channel::frame::MAX_FRAME`; a foot that read a tool's output without a bound
therefore had two failures queued behind a loud tool — an allocation as large
as whatever it cared to print, and then a completion the framing refuses, which
every layer above the frame reads as a dead channel. The foot exited, the
invocation was answered by nothing, and the asking side waited out its whole
slot on a tool that had finished. Three things settle it, and the first is the
only one that is a decision:

- **Truncate in band, do not refuse.** The size is a local fact and the foot is
  holding the bytes, so it answers the shape it already answers for a deadline:
  a bounded capture plus a `thrall:` sentence naming the stream, the limit and
  how many bytes were dropped. A refusal naming the size would be a *second*
  kind of non-answer for a question this box can answer, and REMOTE §5.3's
  capture contract already rules a capture a lossy projection — *"a tool whose
  output is not UTF-8 loses exactly the bytes no string can name"*. Size is
  that same trade in the other axis, and unlike the transcode it can say so.
- **The bound is derived from the frame, not typed beside it.**
  `exec::CAPTURE_LIMIT` is `MAX_FRAME / 16`: two streams, and JSON escaping can
  spend six bytes on one input byte, so a capture at the bound encodes to at
  most twelve sixteenths of a frame whatever a tool prints. One home for the
  fact, and the frame's own refusal becomes unreachable from a tool's output
  rather than being handled twice.
- **It bounds the ANSWER, never the tool.** The pipes keep being read past the
  limit and the bytes are counted and dropped, because a pipe nobody drains
  blocks the tool's next write and would turn a bounded question into a
  timeout. The tool runs to its own end and its exit code is its own.

**A container image does not change that sentence** (bl-3586, under yog
bl-223f). thrall ships an OCI image because an image is a convenient unit of
install, and for no other reason: nothing in thrall uses the container
filesystem as a feature, no state lives in a layer, and running the foot in a
container is not containment thrall enforces and must never be described as
such. Whatever a runtime happens to confine is the operator's arrangement of
their own machine — it is invisible to this code, unstated on the wire, and
outside every claim above. The one thing the image DOES decide is which
binaries a tool entry can name at all, and that is a floor, not a fence: the
runtime layer is a base with a shell precisely because a foot execs
operator-configured argv, so a layer that could confine anything interesting
would be a layer that could run nothing useful. `README.md` states the choice
and its reasoning.

**What the image DOES have to prove is that it carries nothing it should not.**
§3.2 and §3.3 put the leaf and its key outside every channel thrall has; the
image is a channel, and a certificate baked into a layer is a certificate
published to everyone who can pull it. Until bl-7075 that was a sentence in a
comment. `make image-scan` (yog DESIGN §10.1's condition on the registry
ruling, operator ruling 2026-08-30) reads the built image's authored layers and
its config through the same `scripts/leak-rules.sh` table the commit gate uses,
and runs as the last step of `make image` — the same placement, and the same
reasoning, as the pre-commit hook. It answers the question no source gate can:
`make leak-scan` reads the git index, and an image is built from inputs no
commit has. What it does not reach is stated where the rest of the disclosure
posture is (§5.2): it is prevention, local and bypassable, and thrall has no
published artifact for anything late to check.

### 3.6 Version skew is real now

Separately installed ends make version skew possible for the first time. The
handshake carries a protocol version, and a mismatch **refuses fail-closed,
naming both versions**. A foot and an engine that disagree about the wire must
not discover it one field at a time.

The version moves in lockstep with the engine's, and the engine's corpus
ledger is what forces a move. **The pin is 8** (bl-e0f0), and a foot's own
surface moved exactly twice on the way there: PROTOCOL 2 (bl-36f7) is the
worktree lane's bump — the advertised element gained `subject_cwd` and the
invocation gained `cwd`, both optional, both a change to a shape already in
use — and PROTOCOL 8 (yog bl-66d4) is the `wrote` receipt §3.7 consumes. The
five bumps between are seat-facing shapes this crate never decodes: a
conversation row's `failure` (3), the queue row's `flag` (4),
`reply/governing`'s lineage keys (5), `reply/providers`' `effort` and
`priority` (6), `reply/help`'s `surface` (7). **A foot could dial across none
of them**, because the preface is one integer compared for equality — the
version states the engine's *build*, never which frames this end reads.

**And the number is the engine's, so the suite states it as the engine's.**
`src/corpus.rs` (§4) holds the protocol number and the frames of every shape a
foot speaks as literal text copied from yog, and the stand-in engine dials at
that. It has to: while the stand-in wrote its preface from
`channel::hello::PROTOCOL`, both ends of every test were one constant wearing
two names, agreeing at any value — so the pin sat five versions behind a live
engine, unable to open one real channel, with the whole suite green (bl-e0f0).
A fixture built out of the thing it stands in for cannot fail the one way that
matters. Vendoring the corpus as `.rs` rather than as yog's `.json` files is
`Cargo.toml`'s doing: JSON would have to be ruled into the `include` allowlist
to survive a build from the registry, and would then ship a test fixture inside
a released crate.

### 3.7 The set is keyed on the identity, and a working foot is absent

REMOTE §5.1 stores one advertised set per client identity. It is a fact about a
*machine*, so it is not scoped to a connection — which means any connection
bearing this box's certificate may replace it, and the box that is running has
nothing in the protocol that would tell it (there is no version on the set, no
generation, and no receipt on the read that names which set is in force).

The engine closed the half it can see (yog bl-1462, REMOTE §5.1): a second
concurrent follow-class read under one identity is refused, and an
advertisement that would **change** the set in force is refused while that
client holds a parked read. That covers the whole of an idle foot's life,
because an idle foot is parked.

It cannot cover the window this foot opens itself. §3.1's dial-in shape means
one connection per ask, held only while waiting — so while a tool runs, this
box holds no parked read, the engine cannot tell it apart from a machine that
has gone away, and a set replaced in that window would stand until the process
was restarted, with every later invocation refused for a tool that plainly
exists.

**So the set is asserted again at the end of every hand-off** (bl-2d78). The
window is bounded by one tool's runtime instead of by the process lifetime, and
an idle foot pays nothing: no hand-off ends, so nothing is said. The engine
writes only when the set differs, so the ordinary re-assertion is a comparison
and no write.

**And since PROTOCOL 8 it buys knowing too** (yog bl-66d4, consumed in
bl-e0f0). `advertised` carries `wrote`: false when the engine compared, true
when it changed the document. So a re-assertion that WROTE is this box learning
it was disarmed while it was absent — the set it presented was not the set in
force — and the foot **says so** instead of healing in silence, which was the
whole of what bl-2d78 could not close from this end.

Three properties of that reading, and each is a decision:

- **A `true` on the FIRST presentation of a channel says nothing.** Every fresh
  channel presents into whatever the engine happens to hold, and the ordinary
  first presentation writes. Only a presentation made after a hand-off this
  foot just performed can tell a rival from a beginning.
- **It does not end the channel.** The set is back and the tools work; a foot
  that exited here would hand the box to the rival by leaving. Compare the
  *refused* re-assertion, which does end it — there the engine is telling this
  foot that another connection is serving under its name right now, and this
  one is not the machine in force. Two readings of one hazard, two answers.
- **It goes to a sink, not to the return.** This sentence has to be said with
  the channel still up, and nothing that returns can say it. So the sink is a
  parameter (`run::Notice`) for the same reason the executor is, and the one
  effect — writing to this process's stderr — lives in `src/main.rs`, where a
  test reads notices back as values instead. In `fan` each channel's notices
  carry that channel's name, the same prefix its ending sentence carries.
  **Since bl-e834 every sentence goes here**, endings included: a channel's
  return value is read at a join, and a join waits on the siblings (§3.9).

The foot cannot tell a rival from an engine that lost what it was holding, so
the sentence names both readings rather than guessing one.

### 3.8 The redial, and what does not cross one (bl-916d)

REMOTE §5.3 ***reverses the no-reconnect ruling at the channel and keeps it at
the process***: *"a foot **redials its own channels**, with a backoff that
settles rather than spins ... and still **exits when it cannot be a foot at
all**, which is the part supervision was always the right owner of."*

The old ruling's premise was sound and its conclusion did not follow.
**Supervision restarts a process, and this failure does not kill one.** REMOTE
§1's canonical box sleeps, changes network and crosses a relay switch; TCP
drops; the channel's conversation ends with its sentence; and the foot process
stays healthy, serving whatever other engines it holds. A multi-entry box loses
one channel of several and there is no exit code for a supervisor to see. From
that moment the engine believes this box is gone — presence is connection RAM,
correctly — and the box believes it is serving. The operator's symptom is tools
that silently stopped being offered by a live-looking process.

**The trigger is an ending, and there are two kinds** (`run::hold`). The
classification is structural — *who* failed, at *which* leg — and never
textual, because a foot that decided its own lifetime by reading the engine's
prose would be a foot the far end could rewrite by rewording.

| What ended it | Leg | Answer |
|---|---|---|
| The wire | any | dial again |
| The engine refused | the `invocations` read | dial again, past one hold's width |
| The engine refused | `advertise`, `complete` | over |
| An answer no foot gesture can earn | any | over |
| This box's own material | before any dial | over |

**The two refusals are two animals and must never be collapsed.** A refused
READ is REMOTE §5.1's one-reader guard, and after a drop it names *this very
machine*: the predecessor's read does not leave until the engine tries to
answer it, so a redial inside that window is refused by a connection that is
already dying. REMOTE §5.1 states the bound as a contract — *"Its life is the
hold and not the connection's ... `Mailbox::take` drops the claim on the way
out, before the caller writes the answer, so a peer that vanished without a FIN
frees the slot within one hold's width — thirty seconds"* — so the answer is to
wait past that width and ask again. A foot that took the sentence as final
would make the first network blip permanent, which is the failure the reversal
exists to end. A refused ADVERTISEMENT is the opposite reading and keeps
bl-2d78's answer: the engine declines a set that would replace a *serving*
machine's own, so a refusal there is another connection holding this machine's
read with a different set in force. Dialling again would hand it the box by
pretending otherwise.

**The wait is three numbers** (`run::redial`), and each answers one way the
loop could be wrong. It starts at **one second**, because the ordinary case is
a blip already over by the time it is noticed. It **doubles**, and stops at
**sixty-four seconds**, so a box in airplane mode reaches a dial a minute on
its seventh attempt and stays there rather than burning a core. A one-reader
refusal **floors** at thirty-two seconds — one hold's width and two seconds of
margin — while the series advances underneath it, so a refusal that is a
genuine rival rather than a predecessor backs off like anything else instead of
polling at one fixed cadence forever. And a channel that **served** — that had
a read answered, which is the engine having parked this foot for its own hold —
returns the series to its floor: without that, a laptop sleeping nightly would
creep to the cap and stay there for the life of the process, which is the delay
this whole loop exists to shorten. A hammering loop cannot manufacture that
evidence without the engine handing out work, and a foot already dials as fast
as an engine answers reads.

**The loop imposes no deadline of its own, and must not.** The engine side has
no socket timeouts (yog bl-1421, filed and unbuilt), so a dead peer can wedge an
engine thread for a long time and the first dial after a flap may be answered
slowly — by an engine still working out that the previous connection is gone.
The only deadline in the path is the transport one the channel already had
(`channel::READ_TIMEOUT`, two minutes), which sits comfortably above the
engine's thirty-second hold and is a bound on the *socket* rather than on the
wait; and if it does fire it is the wire, which is retryable, so a slow engine
costs this box one more dial rather than the channel. **Nothing here depends on
bl-1421 landing**, and the predecessor floor is the other half of that: a foot
that hammered an engine which has not yet noticed the dead peer would pile dials
into exactly the party that is busy — where the refusal it earns is answered
immediately and parks nothing, but the handshake is spent for a sentence whose
expiry is already known.

**Nothing crosses a redial, and that is the decision rather than an omission.**
A redial makes a NEW channel. Presence re-forms as it does for any fresh
connection, the advertisement rides the connection already, registration is
durable engine-side, and an invocation in flight when the wire died is the
engine's mailbox lease (REMOTE §5.3's at-least-once leg) rather than this loop's
— so the same invocation is handed over again under the id it was first handed
under. Three consequences, and each is deliberate:

- **The disarming is not remembered.** §3.7's notice fires on a re-assertion
  that WROTE, and is silent on a channel's FIRST presentation because every
  fresh channel presents into whatever the engine happens to hold. A redial's
  presentation is a first presentation, so it stays silent — and a foot that
  remembered having been disarmed and read the next one as a rival would cry
  rival on the most ordinary redial there is: the engine restarted, or this
  box's own dying predecessor wrote last. That is the noise that gets the real
  notice ignored. **The healing crosses a redial and the knowing does not**, and
  the healing is the half that matters: the re-presentation restores the set on
  every new connection whether or not anything says so, which REMOTE §5.3 calls
  a coarser instance of the same self-heal.
- **It is not a session resume, and there is nothing to resume.** A foot that
  carried state across the gap would be a foot with a world, which §2 refuses.
- **The sentence has to be said as it happens.** Under a loop the sentence that
  ended a connection is no longer returned by anything, so every retryable
  ending goes to the §3.7 notice sink with the wait beside it, under the
  channel's own name. What `run::fan` returns is the *terminal* sentence — the
  ending another dial cannot improve — and a box whose every channel has ended
  that way is a foot that cannot be a foot, which is where supervision takes
  over. That sentence is now **said as well as returned** (§3.9): the return is
  the exit's, and the saying is the operator's.

**Where the loop sits is the whole of why it is safe** (`run::redial`, between
`Entry::open` and `run::hold`). `fan` already owns the per-channel lifetime, so
the loop is above one conversation and below the box: a redial re-opens no
material, re-reads no config, and shares nothing with the other channels. The
entry is opened once and not per dial, because opening reads no file and dials
nothing — it is a fact about this box's own material, so a failure there is
over rather than retried.

### 3.9 A channel's ending is reported by that channel (bl-e834)

**A return value is read where the reader gets to it, and `run::fan` reads its
threads at a join, in filing order.** So a sentence that is only returned is a
sentence held until every earlier-filed sibling has stopped — which for a
healthy sibling is never. On a one-entry box the process exits at the first
death and nothing is lost; on a multi-entry box the operator's symptom is a
live-looking process serving one engine and silently not serving another.

That is a report that does not ship, and §2 makes the sentence the whole
product: thrall's channel does not reconnect *the process*, so a supervisor's
log is all an operator gets. It also blunts the two refusals yog bl-1462 added,
which are the engine telling this box that something else is presenting its
certificate — the loudest thing the far end can say, arriving at a process that
will not repeat it.

**So each channel says its own terminal sentence the moment it stops**
(`run::served`), through the same §3.7 sink and under the same name `fan` joins
with, and the joined vector stays exactly what it was: the summary the exit is
written from. Four properties are the decision:

- **It is said in the channel's OWN thread**, not in the join loop. Saying it
  at the join is the near miss and it is still buffered — the second channel's
  sentence would wait on the first channel's thread. The suite proves the
  difference: the sibling that is still serving is the one filed FIRST, and its
  wait does not end until the sink has heard the other's ending.
- **It is the same words in both places.** An operator reading a log and an
  operator reading the exit must not have to reconcile two wordings of one
  ending, so the notice carries the line the summary will carry.
- **It is beside the verdict and never instead of it.** The exit code and the
  verdict text are `cli`'s and are tested as values; nothing here changes them.
- **It stays off the pure-function path.** `cli::run` decides and says nothing,
  which is what keeps `src/main.rs` the one file outside the coverage floor.

The one ending no channel says for itself is a **panic**, which unwinds past
the saying: `fan` names it at the join, because a thread that broke had no
sentence to hand over and that outcome is this program failing rather than a
channel ending.

---

## 4. Module map

The map is the design-time split, kept ahead of the 300-line cap rather than at
it. Rows below the line are unbuilt; each names the ball that will build it.

| Path | Role |
|---|---|
| `src/lib.rs` | Crate root. Module declarations and the crate's own statement of what it is. |
| `src/cli.rs` | The command line as a pure function: arguments in, a `Verdict` (exit code + text) out. No process state is touched, which is what lets `main.rs` be the one coverage exclusion without excluding a decision. |
| `src/main.rs` | The process entry and nothing else: argv in, stream selected by the code, exit. The single `tarpaulin.toml` exclusion. |
| `src/channel.rs` | **The channel** (bl-a4a5): one wire to one engine. Dial per ask, hold only while waiting, never reconnect. There is an `ask` and there is nothing else — the shape of the file is the dial-in invariant. |
| `src/channel/frame.rs` | The framing: a big-endian `u32` length, then that many bytes of JSON; a zero-length frame terminates an answer (REMOTE §3). |
| `src/channel/hello.rs` | The version preface, and this end's half of it — state, confirm, refuse fail-closed naming both versions. A foot never *admits*, because a foot is never dialled. |
| `src/channel/tls.rs` | The rustls client configuration: the operator CA as anchors, this box's leaf as its identity, `ring` named rather than defaulted. |
| `src/channel/leaf.rs` | The foot grade, read off this box's own certificate — a DER walk, because thrall links no certificate library. |
| `src/channel/material.rs` | What the operator carried here, and the three answers a directory can give: nothing, half, or a channel. |
| `src/channel/entries.rs` | The entries this box holds, one per channel. A refusal is one entry's, never the set's. |
| `src/config.rs` | **The operator's document** (bl-05fe): what this box offers, and the projection that drops the local half. The gate on what is enabled. |
| `src/tools.rs` | The advertised element — the three facts REMOTE §5.1 fixes, in one spelling spent by the wire and by the document alike, and the check that a set is addressable. |
| `src/json.rs` | The strict field reads every decoder here shares: a missing field, a mistyped one and a wrong-shaped one each refuse with the key an operator typed. |
| `src/gestures.rs` | **The foot set** (bl-a2ea): `advertise`, `invocations`, `complete`, and the answers they can earn. The enumeration is the enforcement thrall can keep — there is no spelling here for a fourth verb. |
| `src/invocation.rs` | What crosses the routing leg: the invocation a foot is handed and the capture it hands back, each in one strict spelling. |
| `src/run.rs` | **The loop**, and the three seams that are parameters: what runs a command (`Handoff`), where a channel says something while it is still serving (`Notice`, §3.7) and where it waits between dials (`Pause`, §3.8). All three are effects no test can read back, so the one implementation of each is `src/main.rs`'s — which is what lets the whole conversation be tested against a real engine, a one-line executor and two recorders. This file itself is `fan`: every channel this box holds, one thread each — and `served`, which says a channel's terminal sentence in that channel's own thread rather than at the join (§3.9). |
| `src/run/hold.rs` | **One channel's conversation** (split from `run.rs` by bl-916d): present, wait, hand off, answer, present again — and the `Ending` that stopped it, classified by who failed and at which leg (§3.8). |
| `src/run/redial.rs` | **One channel's lifetime** (bl-916d): the endings worth another dial, the backoff that settles, and the sentence said in the meantime. It is a separate file because it is a separate question — what happened, against what to do about it — and because its whole decision is three numbers and one line of arithmetic, testable as values. |
| `src/exec.rs` | **The executor's dispatch** (bl-4cda): which entry an invocation names, whose working directory it may run in (§3.4's `subject_cwd`), and the three facts that come back. Every outcome is a capture — a tool that ran, one that overran, a name this box does not carry, a command that would not start. |
| `src/exec/child.rs` | One child, from the fork to the capture: the spawn, the poll that is also the drain, and the cascade that stops a tool which will not stop itself. Split from `exec.rs` when the drain moved into the poll (bl-6c14) — the seam is *deciding what to run* against *running it*. |
| `src/exec/pipes.rs` | The child's three pipes, pumped without blocking and read within a bound (bl-6c14, bl-6028; §3.5). A read answers with what the pipe holds now, so a write end a helper still holds cannot outlast the invocation — and the drains and the input feed stop being threads, because none of them can block any more. Past `exec::CAPTURE_LIMIT` it keeps reading and stops keeping, counting what it dropped so the capture can say so. |
| `src/serve.rs` | What `thrall run` does: read the document, read the channels, serve until they stop. There is no success exit, so none is spelled. |
| `src/paths.rs` | The one data root, named by `$XDG_DATA_HOME` or `$HOME` and by nothing of thrall's own. Neither set is a refusal, never a relative guess. |
| `src/spawn.rs` | **The spawn boundary.** Every child process is built AND forked here — nowhere else builds a `Command`, and nowhere else spends one. It decides three things a spawn site could forget: the git-environment scrub, the **process group** the child is born leading (bl-a78e, §3.5), and the fork lock the suite needs. **Founded by bl-a4a5**, before it had a production tenant, which is the point of the row: a boundary rule that arrives after the first spawn site is a rule that has to be argued with. |
| `src/sys.rs` | **The confined `unsafe` file**, and it holds two things, both raw process effects `std` does not wrap. Signalling a process GROUP, which `std` has no spelling for at all (`Child::kill` is `SIGKILL` to one process, and there is no `Child::terminate`); the sign guard did not move when the group arrived (§3.5) — the negation is this file's, the callers pass a positive id. And putting a pipe into non-blocking mode (bl-6c14), which `std` spells for sockets and for nothing else — a `ChildStdout` has no `set_nonblocking`, and borrowing the socket one by wrapping the descriptor in a `UnixStream` would read the pipe with `recv(2)`, which a pipe refuses. Both are declared rather than depended on: `kill(2)` and `fcntl(2)` are in the libc `std` already links, so neither costs a crate, a build script or a lockfile line. |
| `src/state.rs` | **The lock chokepoint.** Every `Mutex`/`RwLock` in the crate. Unbuilt, and it stayed that way: the only cross-thread hand-offs thrall has are a `JoinHandle`'s own answer (the pipes a child writes, the sentence a channel ends with), which need no lock. The suite's fork lock is **not** a tenant — a test's serialization lock is scaffolding, and the rule's own text sends it to `src/test_support.rs`. |
| `src/corpus.rs` | `cfg(test)` only. **The engine's conformance corpus, vendored as literal text** (REMOTE §3, bl-e0f0): the protocol number and the frames of every shape a foot speaks, copied from yog rather than derived from anything here. `corpus/tests.rs` pays what §3 says a client owes them — decode every frame, and round-trip every request byte for byte. It is the crate's one statement of what the FAR end says, which is why the stand-in engine reads its version from here and not from the pin it is testing. |
| `src/test_support.rs` | `cfg(test)` only. The scratch directory, the fork lock, the stand-in engine, the recording notice sink (§3.7 — a serving foot writes to stderr, and a test cannot read that back), and the certificate mint the suite performs on the operator's behalf. |
| `src/packaged_tests.rs` | `cfg(test)` only. **The publication guard** (bl-d25a): what `cargo publish` would upload, read off the real `cargo package --list` and judged against the classes `Cargo.toml`'s `include` allowlist rules in — both directions, since a shape guard dies by matching nothing. It is in `src` rather than a `tests/` crate because it forks a child and the spawn boundary is `pub(crate)`; an integration crate could only reach a bare `Command::new`, which the confinement rules refuse. |

**There is no flat material root, and its absence is a simplification rather
than an omission** (bl-a4a5). Upstream a client box also holds material
directly under `wire/`, because that same directory is where a *server* keeps
the address it binds. A foot never binds anything (§2), so that second meaning
does not exist here and one shape covers every case: every channel is an entry
at `wire/workspaces/<leaf>/`, and a box with one engine has one entry. The four
file names inside are REMOTE §8.2's, unchanged, so a pair the operator minted
for a client box is filed the same way whichever program reads it.

The last three rows are named by their **confinement rules** (bl-1827) rather
than by the code that will fill them, and the naming is deliberately ahead of
the code. A confinement rule has to name one location before the first site is
written, or the first site picks the location by being written — and a rule
that arrives afterwards is a rule that has to be argued with. Each rule's
`ignores` list is the one location authority for its kind: a site is added to
the named file, never a path to the list, because a second confined file is two
inventories, which is no inventory.

**A rule with nothing in `src` to match cannot be measured by scanning `src`.**
All four confinement rules are still in that state and always will be — their
one lawful site is inside the file each `ignores` — which is how a rule passes
as green forever. They are proved from the other direction instead: `make
rules-audit` runs every rule ALONE, by the id in its own file, against
`rules/fixtures/violations.rs`, and fails the rule that flags nothing.

---

## 5. Deferred directions

Recorded so nobody builds toward them by accident, and so nobody re-litigates
them from scratch.

### 5.1 The MCP bridge (bl-d5d6)

**Deferred, not v1.** thrall runs as an MCP client against MCP servers on its
own box and **re-advertises their tools up the wire** as ordinary entries in
its own advertisement — an MCP tool becomes a `{name, description,
input_schema}` triple like every other, and an invocation routed to it is
dispatched over MCP instead of to an argv.

Two properties are the whole point:

- **The engine never learns MCP.** MCP terminates at the foot. Upstream sees
  one vocabulary and gains no protocol, no verb, and no transport.
- **The local config stays the gate.** An MCP server is enabled the way a
  command is: by appearing in the operator's document. Nothing is discovered
  and auto-advertised.

Open questions are in the ball, not here; the ball is the living document for
work that has not started.

### 5.2 What is deliberately absent from the gate

- **A post-publication scan of the crate on the registry.** The two halves of
  the disclosure gate have both landed and neither reaches this. The early half
  is the scanner (bl-e878): the tree, the commit message and the store op's own
  commit are read by one rule table before anything is written, on the author's
  box, bypassably. The late half is `.github/workflows/store-scan.yml`
  (bl-e95a), which runs that same scanner over the published `balls/tasks` ref
  daily, on dispatch and on a rule-table change — the only check an author
  cannot switch off, detecting rather than preventing, since by then the remedy
  is a history rewrite. `main` needs no such job: CI runs `make leak-scan` over
  that tree on every pull request and every push. What remains absent is the
  **third** published surface bl-006e's publish half opened, and it is the one
  where a hit has no remedy at all — a git ref can be rewritten, a published
  version cannot, and yanking it leaves it downloadable. What stands between a
  leak and that surface is the `include` allowlist and `src/packaged_tests.rs`,
  which judge file CLASSES; the CONTENT of a shipped `src` file is read by the
  tree scan before it is committed and by nothing after. No workflow can close
  that — a scan after the fact would find what cannot be withdrawn — so it is
  the publication checklist's, run by a person, every time. That is worth
  stating plainly rather than implying a gate that reaches further than it does.
Nothing else. The confinement rules that stood here — `unsafe`, the lock
chokepoint, the spawn boundary — landed in bl-1827, ahead of the surfaces they
govern, and §4 says where each one points and why that order is the right one.
The founding's objection was real and is answered rather than waived: a rule
with nothing to measure passes as green forever, so `rules-audit` stopped
measuring rules by scanning `src` and now measures every one of them,
individually, against its own deliberate violation.
