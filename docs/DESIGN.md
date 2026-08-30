# thrall — DESIGN

**Status: founding skeleton.** No wire surface exists yet. This document
states what thrall is, what it may never become, and which invariants it
inherits rather than owns. It is a living document: amend it when reality
diverges, and never code around a stale section.

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
- **It runs one invocation at a time.** Serial, which is what makes a busy foot
  *absent* at the far end — and why presence is not the routing predicate
  there.

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
| — | — |
| *channel* | mTLS dial, leaf presentation, the protocol-version handshake, entry resolution (bl-a4a5). |
| *config* | The operator's tool document; the advertisement is its projection (bl-05fe). |
| *loop* | advertise, the mailbox wait, the hand-off (bl-a2ea). |
| *executor* | Spawn, the two deadlines, the one transcode, the capture (bl-4cda). |

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
- **Three confinement rules (bl-1827).** `unsafe` confinement, the lock
  chokepoint, and the spawn boundary are installed with the surfaces they
  govern, each with its own fixture. A rule with nothing to measure is a rule
  that passes as green forever, and installing it early buys a false signal
  rather than an early one.
