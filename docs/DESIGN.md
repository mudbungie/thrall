# thrall — DESIGN

**Status: thrall is a working foot.** `thrall run` reads this box's tool
document, opens every channel the operator provisioned, presents what the box
offers, waits on its mailbox, runs what comes back and posts each capture — over
a real mTLS channel with a real version preface and a real child process. The
four balls that built it: the transport and the foot-grade check (bl-a4a5), the
operator's document and the advertisement derived from it (bl-05fe), the three
gestures and the loop that spends them (bl-a2ea), the executor and the capture
(bl-4cda).

What remains is deliberately absent: §2's list of what thrall may never become,
§5.1's MCP bridge, and the publication question (bl-006e). This document states what thrall is, what it
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

    advertise → { wait on the mailbox → execute → post the capture } → forever

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
- **It never reconnects on its own.** A channel that fails is an exit naming
  the failure. Restart policy belongs to the supervision the operator's machine
  already has, and inventing one here would be thrall deciding how a box it
  does not administer runs a program.
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

`command` is an **argv, spawned directly**. No shell, and no interpolation of
the invocation's input into it: a shell would make the declared `input_schema`
advisory and turn an operator's config file into a command-injection surface
for anything the model can type.

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

**As built (bl-4cda), here is the whole of it.** thrall enforces exactly three
things locally, and it is not a sandbox:

- **The name**, from the operator's document: a tool absent from it cannot be
  invoked, and what runs is that entry's argv, spawned directly — no shell, and
  no interpolation of the invocation's input into it.
- **The directory**, when the operator named a `cwd`.
- **The deadline**: the child is asked to stop with `SIGTERM`, given a grace,
  and then killed.

Everything else is the box's. The child runs as this process's user, with this
process's environment less the git scrub the spawn boundary performs — no
namespace, no rlimit, no filesystem restriction. And **the cascade signals the
child, not its process group**, so a tool that starts something and returns
leaves that something running past its own deadline. Closing that needs the
child to be a process-group leader and the signal to be sent to the group, which
is a change to how thrall spawns rather than a knob (bl-a78e). Until then the
sentence above is the claim, and it is the whole claim.

### 3.6 Version skew is real now

Separately installed ends make version skew possible for the first time. The
handshake carries a protocol version, and a mismatch **refuses fail-closed,
naming both versions**. A foot and an engine that disagree about the wire must
not discover it one field at a time.

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
| `src/run.rs` | **The loop**: present, wait, hand off, answer — and the fan that serves every channel at once. Execution is a parameter (`Handoff`), which is what lets the whole conversation be tested against a real engine and a one-line executor. |
| `src/exec.rs` | **The executor** (bl-4cda): the tool contract, the deadline and its cascade, the one transcode. Every outcome is a capture — a tool that ran, one that overran, a name this box does not carry, a command that would not start. |
| `src/serve.rs` | What `thrall run` does: read the document, read the channels, serve until they stop. There is no success exit, so none is spelled. |
| `src/paths.rs` | The one data root, named by `$XDG_DATA_HOME` or `$HOME` and by nothing of thrall's own. Neither set is a refusal, never a relative guess. |
| `src/spawn.rs` | **The spawn boundary.** Every child process is built AND forked here — nowhere else builds a `Command`, and nowhere else spends one. **Founded by bl-a4a5**, before it had a production tenant, which is the point of the row: a boundary rule that arrives after the first spawn site is a rule that has to be argued with. |
| `src/sys.rs` | **The confined `unsafe` file**, and it holds exactly one thing: `SIGTERM`, which `std` has no spelling for (`Child::kill` is `SIGKILL`). Declared rather than depended on — `kill(2)` is in the libc `std` already links. |
| `src/state.rs` | **The lock chokepoint.** Every `Mutex`/`RwLock` in the crate. Unbuilt, and it stayed that way: the only cross-thread hand-offs thrall has are a `JoinHandle`'s own answer (the pipes a child writes, the sentence a channel ends with), which need no lock. The suite's fork lock is **not** a tenant — a test's serialization lock is scaffolding, and the rule's own text sends it to `src/test_support.rs`. |
| `src/test_support.rs` | `cfg(test)` only. The scratch directory, the fork lock, the stand-in engine, and the certificate mint the suite performs on the operator's behalf. |

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

- **The late half of the disclosure gate.** The scanner landed (bl-e878): the
  tree, the commit message and the task store are all read by one rule table
  before anything is written. What does not exist is the check that runs
  *after* publication — a scan of the published ref, which is the only one an
  author cannot switch off. It needs a remote, and thrall has none (bl-006e).
  Until then thrall's disclosure posture is prevention only: local, and
  bypassable by whoever runs it. That is worth stating plainly rather than
  implying a gate that reaches further than it does.
Nothing else. The confinement rules that stood here — `unsafe`, the lock
chokepoint, the spawn boundary — landed in bl-1827, ahead of the surfaces they
govern, and §4 says where each one points and why that order is the right one.
The founding's objection was real and is answered rather than waived: a rule
with nothing to measure passes as green forever, so `rules-audit` stopped
measuring rules by scanning `src` and now measures every one of them,
individually, against its own deliberate violation.
