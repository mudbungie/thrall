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
is `line-cap → clippy -D warnings → rules-audit → cargo deny check`. The
pre-commit hook runs the same targets via `scripts/pre-commit`; neither
restates a step the Makefile defines. Run `make install-hooks` once per clone.

**All tests must pass and coverage must be 100% before anything merges.** It
does not matter who broke the test.

Two things the gate does NOT yet cover, and both are filed:

- **No disclosure scanner** (bl-e878). Nothing mechanical reads what your task
  bodies publish. The rule below is entirely on you until it lands.
- **Three confinement rules are absent** (bl-1827): `unsafe`, the lock
  chokepoint, the spawn boundary. Each lands with the surface it governs. Do
  not install one vacuous to look thorough — a rule with nothing to measure
  passes as green forever.

## Task tracking

Tasks are `bl` (balls). Run `bl --skill` before using it, and
`bl <command> --skill` before running a command.

- Session start is `bl prime --as YOUR_IDENTITY`, then `bl list`.
- **Claim → work → close, in the worktree.** `bl claim <id> --as ID` prints a
  `work/<id>` worktree; **every edit goes there**, never on `main`. A stray
  edit on `main` is invisible to the squash and is left behind. Always pass
  `--as ID` — never let the model invent a name.
- The store is founded **stealth**: `task-remote` is the "no remote, on
  purpose" sentinel, so no op pushes or discovers anything. Setting a remote is
  an operator decision (bl-006e), and the publication checklist in that ball
  applies in full before a first push.

## What may never enter a ball body

A ball body is markdown on a git branch that publishes with the source. Nothing
you write in one is private, and no gate is reading it. Write the reasoning;
leave out the identity, the chronology and the machine state:

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

## Never

- Never credit AI or tooling in commit messages, code, or docs.
- Never `cargo publish`. `publish = false` is the enforcement; the registry
  name is held by a placeholder and is not to be touched (bl-006e).
- Never add a dependency without explicit approval (bl-e5ba).
