# thrall — Agent Operating Guide

You are working in **thrall**, a single binary crate: the **foot** of the
four-component split — a tool-execution client that dials in to a yog server,
advertises what this box offers, waits on its mailbox, and posts captures back.

Read `README.md` first for what thrall is and how to build it. This file is the
discipline that surrounds the work.

## Authorities, and they do not overlap

- **yog's `docs/REMOTE.md` is the PROTOCOL authority.** It is versioned and all
  four components implement against it. thrall does not get a vote on the wire.
- **`docs/DESIGN.md` is thrall's ARCHITECTURE authority** — the role, the
  inherited invariants, the module map. Amend it when reality diverges; never
  code around a stale section, and never implement a deviation silently.
- **`README.md` states the code-style rules**, and `rules/`, `Cargo.toml
  [lints]`, `clippy.toml` and `deny.toml` enforce what a machine can.

Where code and an authority disagree, one of them is a bug. Do not invent a
third answer.

## The gate

`make check` is the complete gate: `fmt-check → lint → coverage`, where `lint`
is `line-cap → leak-scan → clippy -D warnings → rules-audit → cargo deny
check`. The pre-commit hook runs the same targets via `scripts/pre-commit`;
neither restates a step the Makefile defines. Run `make install-hooks` once per
clone — it seats `pre-commit` **and** `commit-msg`.

**All tests must pass and coverage must be 100% before anything merges.** It
does not matter who broke the test.

### The disclosure gate

`make leak-scan` is the disclosure half (bl-e878, ported from yog).
`scripts/leak-rules.sh` is the one definition of what may not be committed —
private keys, vendor tokens, credential assignments, routable IPv4/IPv6/MAC
addresses, absolute paths under any home root on any platform (the synthetic
roots `/home/u`, `/home/op`, `/home/x` are the only account names that pass),
email addresses outside the reserved documentation space, dialogue behind a
speaker label, agent-session artifacts, credential-shaped file paths, and
**content no rule can read**. `scripts/leak-scan.sh` is the mechanism;
findings are truncated to 12 characters, because a finding must LOCATE a leak,
never reprint it into a terminal or a log.

Four properties are load-bearing and none is decoration:

- **It reads index BLOBS, not the worktree**, so the bytes scanned are the
  bytes committed — a leak staged and then overwritten with a clean copy on
  disk is still caught.
- **Unreadable is rejected, not skipped.** `grep -I` silently passes binary
  files, which is the class most likely to carry a dump. thrall's allowlist of
  tracked binaries is EMPTY; a derivation is added to it only with a test that
  regenerates it and checks it byte for byte.
- **Both directions, run first.** `--self-test` is the stronger half: every
  rule owns a fixture in `scripts/leak-fixtures/` where every non-comment line
  must be flagged *by that rule* and must carry the `notreal` marker, plus
  near-misses that must NOT be flagged. A leak gate dies by matching nothing
  and passing everything forever; a noisy one dies by being bypassed.
- **`.githooks/commit-msg` runs the same scanner over the commit MESSAGE**,
  which no pre-commit step can see.

Two scopes, because they answer different questions. Bare, it scans the whole
tracked tree — the right question for a commit hook, because the tree IS your
change. `--commit REV` scans what one commit publishes: the blobs it adds or
rewrites plus its message, which is the store gate's question and the only
scope that can read a `-m` note.

**A second artifact needs a second gate: `make image-scan`** (bl-7075, under
yog DESIGN §10.1). The scan above reads the git INDEX, and an OCI image is
built from inputs no commit has — the build context as the engine actually
receives it, the base layers, the package index, and the image CONFIG. The
image gate reads all three surfaces through **this same rule table**, is a step
of `make image` rather than a target beside it, and runs both directions (a
scratch image with a planted secret in a layer, another in an `ENV`, and an
undeclared binary, all of which must be caught, before the real image is
scanned). `README.md` §"The image-side disclosure gate" states its mechanism.
**The build context is the image's `exclude` list** — `Containerfile` `COPY`s
by name and `.containerignore` keeps the rest from being sent at all — so the
one question a publication checklist asks is now asked once per channel, and
`make image-scan` is the container half's answer to `cargo package --list`.

**What no hook can promise.** This scans one tree. Old commits, other refs,
and anything published elsewhere are outside it — and thrall now HAS published
refs (bl-006e's remote half landed; the trunk and the task store go to the same
remote), while the late half (a scan of what is actually public) still does not
exist: bl-e95a. So thrall has prevention only: local, and bypassable by whoever
runs it.

### The confinement rules, and the three files they name

`make rules-audit` runs the `rules/` table (bl-1827 completed it). Four rules
confine a kind of code to one file, and all four name a file that **does not
exist yet** — which is the point: a confinement rule installed after the first
site is a rule that has to be argued with.

| Kind | Confined to | Rule |
|---|---|---|
| `unsafe` block or `unsafe fn` | `src/sys.rs` | `unsafe-outside-sys.yml` |
| `Mutex` / `RwLock` | `src/state.rs` | `locks-outside-state.yml` |
| Building a child (`Command::new`) | `src/spawn.rs` | `no-bare-command.yml` |
| Forking one (`.spawn/.output/.status/.exec`) | `src/spawn.rs` | `no-bare-fork.yml` |

**Each rule's `ignores` list is the one location authority.** Add a site to the
named file; never add a path to the list. A second confined file is two
inventories, which is no inventory. The three files are rows in DESIGN §4.

The spawn boundary is two rules because building and forking are two contracts:
one decides what environment a child inherits, the other holds the ETXTBSY
window a fork opens on every *other* thread's open write fd. Neither has a test
carve-out — `#[cfg(test)]` is where the fork hazard actually lives.

**The audit is per RULE, not per directory.** It runs each rule alone, by the
id in its own file, against `rules/fixtures/violations.rs`, and fails the one
that flags nothing. That is what makes a rule with nothing in `src` to match
measurable at all: scanning `src` is silent about the four above whether they
work or not. A new rule with no fixture fails on the run that adds it.

## Task tracking

Tasks are `bl` (balls). Run `bl --skill` before using it, and
`bl <command> --skill` before running a command.

- Session start is `bl prime --as YOUR_IDENTITY`, then `bl list`.
- **Claim → work → close, in the worktree.** `bl claim <id> --as ID` prints a
  `work/<id>` worktree; **every edit goes there**, never on `main`. A stray
  edit on `main` is invisible to the squash and is left behind. Always pass
  `--as ID` — never let the model invent a name.
- The store **publishes**. It was founded stealth — `task-remote` held the "no
  remote, on purpose" sentinel — and bl-006e's remote half cleared it: the task
  store rides to the same remote as the source, so a ball body is published
  text the moment it is written. What that means for what you may write in one
  is below.

## What may never enter a ball body

A ball body is markdown on a git branch that publishes with the source. Nothing
you write in one is private, and the source gate has never seen a byte of it —
`make leak-scan` reads the index of *this* tree, and the store is not in it.
The task store gate below closes the mechanical half; the half no regex can
reach is yours. Write the reasoning; leave out the identity, the chronology and
the machine state:

- **Other people's names, handles and addresses.** Third parties, other
  operators, anyone who did not publish themselves. The maintainer's own
  published identity is fine; every other address is a leak.
- **Verbatim transcript prose.** Operator dialogue, model output, an agent's
  own reply pasted back in. Cite the conclusion and the ball it came from — a
  conversation is content somebody said, and quoting it publishes them.
- **Live machine state.** Process ids, load figures, absolute paths under a
  real home, host and device names. Cite the *shape*, not the instance.
- **Provider auth state.** Who is signed in to what, which credential exists.
  "The account cannot run jobs" is the fact; the provider's sentence about it
  is disclosure.
- **Conversation and session ids.** Vendor resource ids, transcript keys, the
  identifiers of a specific run on a specific box.

### The task store gate

`scripts/thrall-leak-gate` is a **balls plugin** that runs
`scripts/leak-scan.sh --commit` over **the op's own store commit** — the same
script and the same rule table as `make leak-scan`, because two copies of the
rules drift within a week — and exits non-zero. A non-zero exit is the balls
protocol's abort: the op is refused and the plugins that already ran roll back
in reverse, so the store commit is un-sealed before anything can publish it.

It hangs at `<op>.post`, not `pre`. A `pre` plugin runs **before** `bl` writes
the task file, so it would scan the previous state and wave through the very
body being added; `post` is the one window in which the ball exists and has not
yet been published.

It scans the op's COMMIT, not the store. A store checkout is shared and
long-lived: a tree scan there judges every agent for every other agent's text,
so one polluted body refuses every `bl` op in the checkout — `create` included,
so the defect about the wedge could not be filed. The author who writes a bad
body is the one who should be told, at the moment of writing.

**A machine layer may already be running this scan, and then the wiring below
is a duplicate** (bl-47f5). An operator's box can carry one balls plugin named
at machine scope — every landing, present and future — that keys OPT-IN on the
project's own signal: a repo shipping an executable `scripts/leak-scan.sh` gets
its store scanned before the tracker publishes it, and a repo without one
passes silently. thrall opted in the moment that scanner landed (bl-e878), so
on such a box the store gate is already running on every publishing op and
`bl install`ing the plugin below seats a second copy of the same scan. **The
recipe is the fallback for a box with no machine layer, not a step to always
run** — check `bl conf` for what is already scheduled before wiring anything.
What the machine layer needs to be equivalent is the SCOPE: `--commit` off the
§7 payload's top-level `commit` field, because bare is the whole-store scan and
the shared-checkout wedge above. That plugin is not a thrall artifact and this
repo cannot edit it; this paragraph is the record of what it must pass.

Wiring is one act per checkout, and this repo cannot perform it — the plugin
schedule lives in the balls landing (`balls/config`), not in thrall's tree:

    bl install --bin thrall-leak-gate=<repo>/scripts/thrall-leak-gate
    for op in create update claim unclaim close drop; do
      bl conf prepend $op.post thrall-leak-gate
    done

`prepend`, never `append`: plugins run in list order and only the irreversible
belongs last, so the gate must sit ahead of whatever publishes and whatever
squashes. Those six ops are exactly the ones a publisher runs on — *the gate
goes immediately before the publisher, everywhere the publisher runs*. It is
severable: `bl conf remove <op>.post thrall-leak-gate` deletes config, not code.

**What it cannot do.** It stops the accident, not the author: the same agent can
`bl conf remove` it, or commit inside the store clone by hand, exactly as
`git commit --no-verify` defeats the source hook. There is no unbypassable
preventive placement to move it to — a git hook inside the store clone is
strictly worse (untracked, per-clone, re-founded by `bl prime`, absent on every
other box and silently so). Upstream's answer to that residual is a scheduled
scan of the PUBLISHED ref, which detects rather than prevents; thrall now has
the remote that check needs and not yet the check itself (bl-e95a).

## Never

- Never credit AI or tooling in commit messages, code, or docs.
- Never `cargo publish`. `publish = false` is the enforcement; the registry
  name is held by a placeholder and is not to be touched (bl-006e). What the
  flip would need is already built and none of it publishes anything
  (bl-d25a): `Cargo.toml` declares an anchored `include` **allowlist** — never
  an `exclude`, because a missing `include` entry costs a build and a missing
  `exclude` entry costs a publication that cannot be recalled — `src/packaged_tests.rs`
  judges the real `cargo package --list` against it in both directions, and
  `.github/workflows/release-plz.yml` carries the release shape with no
  automatic trigger and a job that refuses. Arming it is an operator's edit and
  its checklist is that file's header.
- Never add a dependency that is not on the approved set. **`Cargo.toml`'s
  `[dependencies]` comment is that set** (bl-e5ba, approved 2026-08-29): rustls
  with `ring` and no default features, `serde_json`, `thiserror` — each landing
  in the manifest with the ball that first LINKS it, never in advance. `clap`
  is deferred pending a verb surface that needs it; `tokio` and `rcgen` are
  refused outright. Anything else needs a fresh approval, and the reasons live
  beside the list rather than here.
