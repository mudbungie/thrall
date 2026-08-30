# thrall

**The foot.** A tool-execution client for a yog server.

A thrall runs on a machine, holds an operator-issued certificate for it, and
dials in to an engine somewhere else. Its entire wire surface is three acts:

1. **advertise** — present what this box offers: for each tool, a name, a
   description, and a JSON Schema for its input.
2. **wait** — hold open one read on its own mailbox until the engine hands it
   an invocation.
3. **post the capture** — run the thing, and answer with what came back.

That is the whole of it. A thrall asks nothing and acts on nothing. It cannot
start a conversation, read a transcript, spawn an agent, or address any part of
the server other than its own queue. It never listens on a port; **the engine
never speaks first**.

## Why it is a separate program

thrall is one of four components that meet only at the wire — the server, the
seat, the engine, and the foot. Execution was severed from the server so that
the machine holding the conversations is not the machine running the commands,
and so that adding execution to an installation is an explicit act rather than
a property of having installed anything.

A server with no enrolled thrall is a valid, and the default, installation. It
is structurally incapable of executing anything until a foot is enrolled — even
a single-box install enrols its local foot deliberately.

## Foot-grade trust

A thrall's certificate is **foot-grade**: it may advertise and execute, and
nothing else. The grade is binary, not a policy layer — it is not a per-tool or
per-verb access list and is not the beginning of one.

Two honesty clauses ride with that, and neither is decoration:

- **Local config gates what is enabled.** One operator-authored document on
  this box says what thrall offers. A tool absent from it is a tool this box
  does not have. Server-side adjudication is unchanged and stacks on top of it,
  and fails closed.
- **The server cannot inspect this box.** Execution happens on a machine the
  adjudicator cannot see into. Adjudication judges the invocation and nothing
  more; any containment beyond that is whatever thrall enforces locally.
  Neither end may claim otherwise.

## Status

**A working foot.** `thrall run` serves every channel this box is provisioned
for: it dials each engine over mTLS with the protocol version stated by both
ends before either reads, refuses to open at all on a certificate that is not
foot-grade, presents what the operator's tool document enables, waits on its
mailbox, runs what comes back and posts the capture. It does not return while a
channel is up, and it never reconnects — a channel that fails is an exit naming
it, because restart policy belongs to this machine's own supervision.

Two files, both put there by the operator's hand and neither ever written by
thrall: `<data root>/tools.json` and one directory per channel under `<data
root>/wire/workspaces/`, where the data root is `$XDG_DATA_HOME/thrall` or
`$HOME/.local/share/thrall`. A box with neither is a foot that refuses and says
which file is missing.

Certificates arrive out of channel, by the operator's hand, and thrall mints
nothing — there is no bootstrap flow and there must never be one.

`Cargo.toml` carries `publish = false`. The registry name is held by a
placeholder release and is not to be touched: whether, when, and as what thrall
first ships is an operator decision that has not been made, and `cargo publish`
is irreversible — a yanked version stays downloadable. The flag is the
enforcement rather than a note, because a note is not a gate.

## Build

`make` is the build authority. `make check` is the whole gate and nothing runs
a step it does not:

```
make check     # fmt-check -> lint -> coverage
make build     # debug build
make test      # cargo test
make install   # release build, then the binary into ~/.local/bin
make image     # the OCI image (podman or docker), tagged from the crate version
```

`make lint` is `line-cap`, then `leak-scan`, then `cargo clippy --all-targets
-- -D warnings`, then `rules-audit`, then `cargo deny check`.

Every tool is pinned, or the gate is not reproducible: rustc 1.95.0
(`rust-toolchain.toml`), ast-grep 0.44.1 (`sgconfig.yml`), cargo-deny 0.20.2
(`deny.toml`), cargo-tarpaulin 0.35.2 (`tarpaulin.toml`).

Run `make install-hooks` once per clone to seat the pre-commit hook.

## The image

`make image` builds an OCI image from `Containerfile`. **The image is the unit
of install and nothing more** — no part of thrall uses the container filesystem
as a feature, and the container is not a containment boundary. A foot runs what
its operator's tool document says to run; the server still cannot see into this
box, and what stops a tool doing something is whatever this machine enforces
locally, exactly as *Foot-grade trust* above says.

It builds under the pinned toolchain (`rust:1.95.0-alpine`, checked against
`rust-toolchain.toml` during the build so the two pins cannot drift) and copies
one static-pie musl binary into an `alpine` runtime layer. About 12 MB.

**The runtime base is a decision.** A binary that execs nothing can ship `FROM
scratch`, and a statically linked thrall is that binary — until it does the one
thing it exists for. A foot execs operator-configured argv: what it runs is
named in `tools.json` on the box, is not knowable from this repo, and is
routinely a shell line. `scratch` would ship a foot that answers `--version`
and then fails every invocation it was installed to serve. So the layer is
alpine — a shell, a package manager the operator can add their tools with, and
system CA roots for the ones that speak HTTPS. **The floor is provided; what
can run on it is still the operator's problem.** A tool document naming a
binary this layer does not have is a tool this box does not have.

`make image` **pushes nothing**, and there is no `push` target. Where these
images publish is unanswered (yog bl-223f); a push is not undoable, and a
convenience target for an irreversible act is how the act happens by accident.

### What mounts where

The XDG contract is the runtime contract and **the image carries no state**.
`XDG_DATA_HOME` is set to `/state`, which puts thrall's data root — the same
`$XDG_DATA_HOME/thrall` the usage text names — at `/state/thrall`. The extra
level is XDG's and not the image's: `XDG_DATA_HOME` is a parent of
per-application roots by definition.

```
podman run --rm -v /path/to/provisioned/root:/state/thrall:Z thrall:0.0.1
```

That directory is the operator's provisioned data root: `tools.json`, and one
directory per channel under `wire/workspaces/`. Both are put there by hand,
neither is ever written by thrall, and **neither is in the image** — a
certificate baked into a layer is a certificate published to everyone who can
pull it.

There is no `VOLUME` instruction on purpose. A `VOLUME` would let an unmounted
run succeed against an empty anonymous volume; without one, the run refuses and
names the file it could not find, which is the answer an operator can act on:

```
$ podman run --rm -v "$(mktemp -d)":/state/thrall:Z thrall:0.0.1
thrall: /state/thrall/tools.json: No such file or directory (os error 2) — this box has no tool config
$ echo $?
1
```

### What the image deliberately does not contain

- **No certificates and no tool document.** Both are operator-provisioned, and
  both are the reason there is a mount instead of a layer.
- **No `cargo`, no compiler, no source, no `target/`.** The build stage is
  discarded whole; only the binary crosses.
- **No supervisor, no restart wrapper, and no entrypoint script.** `thrall run`
  never reconnects — a channel that fails is an exit naming it, because restart
  policy belongs to this machine's own supervision, and putting a retry loop in
  the image would take that decision away from the operator who has to live
  with it.
- **No bootstrap of any kind.** thrall mints nothing (REMOTE §1.4, DESIGN
  §3.3); an image that could provision itself would be exactly the flow that
  must never exist.

## The rules

Two are hard and machine-enforced:

- **300 lines** on every source file, inline tests included. Docs and config
  are exempt. `make line-cap` is the one definition of both. 300 is a wall, not
  a target; `make line-cap LINE_CAP=199` lists the pre-split band.
- **100% test coverage.** If it can't be tested, it mustn't be built.

A third is hard and machine-enforced but is not about the code:

- **Nothing discloses.** `make leak-scan` reads the index this commit would
  publish — not the worktree — for credentials, routable addresses, home
  paths, pasted dialogue, session artifacts and content no rule can read.
  `scripts/leak-rules.sh` is the one definition of what counts, and
  `--self-test` proves every rule still bites in both directions before the
  tree is scanned at all. `.githooks/commit-msg` runs it over the commit
  message, which no pre-commit step can see. It scans one tree and promises
  nothing about anything already published.

Beyond those, thrall follows the house **contained Rust** standard from birth:
complexity lives in function bodies, where the compiler catches it, not in type
signatures, where it is viral. No named lifetimes; a `pub fn` returns an owned
concrete type; no generic bounds on a `pub` item; no panic paths outside tests;
no `#[allow]` in prod — policy lives in `Cargo.toml [lints]`, justified in one
place; no `Rc`/`RefCell`. Four more rules **confine** a kind of code to one
file: `unsafe` to `src/sys.rs`, `Mutex`/`RwLock` to `src/state.rs`, and both
building and forking a child process to `src/spawn.rs`. Two of the three are
still unbuilt, and that is the discipline working — a confinement rule names its
location before the first site is written, or the first site picks the location
by being written. `src/spawn.rs` was founded the moment the suite needed to run
one program, which is the order the rule exists to force.

The `rules/` directory enforces what it can, and `make rules-audit` checks both
directions: the tree clean **and** every rule, run alone by its own id, still
flagging its deliberate violation in `rules/fixtures`. Per rule rather than per
directory, because nine live rules would answer for a tenth dead one forever —
and because the four confinement rules have nothing in `src` to match yet, so
their fixture is the only thing proving they work at all.

## Authorities

- **`docs/DESIGN.md`** — thrall's own architecture: the role, and the
  invariants inherited from the wire.
- **yog's `docs/REMOTE.md`** — the **protocol authority**. It is versioned, and
  every component implements against it. Where this crate and that document
  disagree, one of them is a bug; never invent a third answer.

Tasks are tracked with `bl`. Run `bl --skill` before using it.

## License

MIT.
