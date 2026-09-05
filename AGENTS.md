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
check`. The pre-commit hook runs the same targets via `scripts/pre-commit`, and
so does `.github/workflows/ci.yml`, which readies a runner and then runs
`make ci`; none of the three restates a step the Makefile defines. Run
`make install-hooks` once per clone — it seats `pre-commit` **and**
`commit-msg`.

CI runs on every pull request, and on `main` as the workflow `release-plz.yml`
CALLS inside its own run — which is where a release is gated (bl-bbb3, and that
file's header says why a called workflow and never a `workflow_run`).

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
  and passing everything forever; a noisy one dies by being bypassed. A fixture
  that does not read as **text** in this locale is reported as an
  infrastructure fault in its own sentence, never as a dead rule (bl-da2a):
  `scan_rule` greps with `-I`, which reports no hits for a file grep judges
  binary and says nothing about why, so without that arm the box's fault and
  the gate's fault arrive as the same sentence.
- **A `grep -q` reads from a herestring, never from a pipe** (bl-da2a), and the
  self-test holds every tracked bash script under `scripts/` and `.githooks/`
  to it — `scripts/thrall-leak-gate` is POSIX `/bin/sh`, which has neither the
  option that makes the shape wrong nor the herestring that fixes it, and is
  skipped by its `#!`; a file with **no** `#!` is a sourced bash fragment and
  is in scope. A piped `grep -q` exits the moment it matches and closes the
  read end; the writer dies of SIGPIPE mid-write, and `pipefail` reports the
  pipeline failed *because the pattern matched* (`PIPESTATUS` reads `141 0`).
  It flaked the self-test into calling a live rule dead, and at `scan_paths` —
  where the shape is `&& report` — it would have dropped a real finding
  instead. The ban is on the shape, not on the option, because a sourced file
  cannot see whether its caller set `pipefail`; and enumerating **zero**
  scripts fails outright, because a check that matches nothing is broken, never
  a clean tree. Measured on yog bl-e33a, the original.
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

**What no hook can promise, and what the late half now does.** A hook scans one
tree, on the author's box, before a push — so old commits, other refs and
anything already published are outside it. Two refs publish to this remote
(bl-006e): `main`, which CI judges with the same `make leak-scan` on every pull
request and on every push, and `balls/tasks`, which until bl-e95a nothing read
after the fact. `.github/workflows/store-scan.yml` is that reading: the repo's
own `scripts/leak-scan.sh` and rule table — never a copy — over the published
store ref, daily, on `workflow_dispatch`, and on any push to `main` that touches
the scanner, the rule table or the mode table, because an edited rule re-judges
a store that has not moved. It DETECTS rather than prevents: by the time it runs
the material is on the remote and the remedy is a history rewrite. What it buys
is that it is the one check the agent writing a ball cannot switch off.
Prevention stays local and bypassable; a published crate version is a third
surface neither half reaches, and it belongs to the publication checklist.

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

**The standing ruling — no agent-session URL in this repository's published
text, anywhere** (bl-d747, operator 2026-08-30: *ban them, no reason to allow
it*). The list above governs a ball body; this governs the published text no
gate can reach. Pull-request titles and bodies, issue text, review comments and
release notes never carry a session URL or a conversation identifier, and the
harness convention of appending one to a pull-request body is **overridden
here** — strip it before you open the PR, because a body cannot be un-published
afterwards: the forge keeps a body's edit history and serves
`refs/pull/<n>/head` forever, so an edit buys the false assurance a history
rewrite buys elsewhere. The ruling came from the seat repository, where exactly
that happened and is now permanent. `session-artifact` reads both forms since
bl-d747 — the bare session id and the code-session URL path shape — so a commit
message or a ball body carrying one is refused at the moment of writing; a PR
body is in no tree, and that half is yours.

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
other box and silently so). The answer to that residual is a scheduled scan of
the PUBLISHED ref, which detects rather than prevents, and thrall now has it:
`.github/workflows/store-scan.yml` (bl-e95a) runs this same scanner over
`balls/tasks` daily, on dispatch, and whenever the rule table changes.
Prevention is local and bypassable, enforcement is remote and late, and stating
that is worth more than a gate that implies otherwise.

## Never

- Never credit AI or tooling in commit messages, code, or docs.
- Never `cargo publish` **on your own initiative**. This is no longer a flag's
  job: bl-006e was adjudicated and 0.0.1 was published by hand, so
  `Cargo.toml` carries `publish = true` and the registry name is this crate
  rather than a placeholder to protect. What that flip bought is a smaller
  irreversibility, not none — a further version is one command away and a
  yanked version stays downloadable — so **a publication is an operator's act,
  every time, and the publication checklist in bl-006e is run in full before
  it.** Two guards stand under that and neither is optional: `Cargo.toml`
  declares an anchored `include` **allowlist** — never an `exclude`, because a
  missing `include` entry costs a build and a missing `exclude` entry costs a
  publication that cannot be recalled — and `src/packaged_tests.rs` judges the
  real `cargo package --list` against it in both directions on every gate run.
  `.github/workflows/release-plz.yml` is ARMED as of bl-bbb3, which MOVES the
  operator's act rather than removing it: a push to `main` keeps one release PR
  open that bumps the version and nothing else, and **merging that PR is the
  control point** — the release job then tags, releases and publishes, behind
  `needs: ci`, so only a green tree ever ships. So do not merge a release PR on
  your own initiative either, and never reach for `cargo publish` by hand: the
  registry accepts a trusted publisher, the workflow performs the OIDC exchange
  itself, and no long-lived registry credential is stored in this repository.
- Never add a dependency that is not on the approved set. **`Cargo.toml`'s
  `[dependencies]` comment is that set** (bl-e5ba, approved 2026-08-29): rustls
  with `ring` and no default features, `serde_json`, `thiserror` — each landing
  in the manifest with the ball that first LINKS it, never in advance. `clap`
  is deferred pending a verb surface that needs it; `tokio` and `rcgen` are
  refused outright. Anything else needs a fresh approval, and the reasons live
  beside the list rather than here.
