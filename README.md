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
mailbox, runs what comes back and posts the capture. It presents its set again
at the end of every hand-off, and **says so on stderr when that presentation
actually changed anything**: the engine's receipt distinguishes a comparison
from a write, so a write there means something replaced this box's advertised
set while a tool was running. It restores it and carries on — the notice is the
part a person needs. It does not return while a
channel is up.

**A channel that drops is dialled again.** A laptop that sleeps, changes
network or crosses a relay switch loses TCP, and until this landed that
engine's tools were silently gone for the life of the process — a live-looking
foot offering nothing, because supervision restarts a *process* and a dropped
channel does not kill one. So each channel takes itself up again, with a wait
that starts at a second, doubles, and stops at a minute, so a box in airplane
mode settles to a slow cadence rather than burning a core. Two refusals are
told apart and never collapsed: the engine declining this box's *read* is its
own predecessor's connection still dying, which frees within one hold's width
and is waited past; the engine declining what this box *offers* is another
connection serving under its name, and that ends the channel. **Restarting the
process is still this machine's own supervision** — a foot that cannot be a
foot at all exits and says why.

Two files, both put there by the operator's hand and neither ever written by
thrall: `<data root>/tools.json` and one directory per channel under `<data
root>/wire/workspaces/`, where the data root is `$XDG_DATA_HOME/thrall` or
`$HOME/.local/share/thrall`. A box with neither is a foot that refuses and says
which file is missing.

Certificates arrive out of channel, by the operator's hand, and thrall mints
nothing — there is no bootstrap flow and there must never be one.

thrall is published to crates.io. It was not, for as long as the decision of
whether, when and as what it first ships was open: `Cargo.toml` carried
`publish = false` and the flag was the enforcement rather than a note, because
a note is not a gate. That decision has been made and 0.0.1 shipped, so what is
left is the part the flag never covered — `cargo publish` is irreversible and a
yanked version stays downloadable, so **what ships is decided deliberately**
— but the deliberate act is the work, not a click. Arming the release workflow
first MOVED the act to merging a release PR; the standing ruling of 2026-09-03
then took that click away too, on the ground that the PR asks a question the
merged work already answered. So a push to `main` keeps one release PR open
proposing the next version and **merges it in the same run, once the gate is
green** (`merge-release-pr` in `release-plz.yml`, whose header carries the
derivation). Nothing untested ships: the merge waits on the very `ci` job the
release job waits on, and the merged bump is published by a second run of the
same gated workflow. Publication authenticates to the registry by a trusted
publisher — a short-lived token minted per run for one workflow file in one
repository — so this repository stores no registry credential at all.

What guards a version's CONTENT was built before the first flip and stands
unchanged. `Cargo.toml` declares an anchored `include` **allowlist** — never an
`exclude`, because a missing `include` entry costs a build, which is loud and
reversible, while a missing `exclude` entry costs a publication that cannot be
recalled — and `src/packaged_tests.rs` reads the real `cargo package --list`
and fails on any path outside the ruled-in classes, in both directions. Without that list
the tree packages whole: the agent guide, the design document, every rule and
hook, and a corpus of deliberately fabricated secrets, shipped beside the
binary. The guard judges file CLASSES and never content; auditing the list
itself stays a human act.

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

`.github/workflows/ci.yml` is the same gate on a runner: it installs those pins
and runs `make ci`, restating no step. It fires on every pull request, and on
`main` as the instance `release-plz.yml` calls inside its own run — so a push to
`main` runs the gate exactly once, and a release is gated on that run.

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

`make image` **pushes nothing**, and there is no `push` target. The registry is
named — `ghcr.io/mudbungie/thrall`, one package per repo, pushed only from that
repo's release workflow at tag time, and what publishes is the version tag and
the manifest digest, both immutable, never a moving `latest` (yog
`docs/DESIGN.md` §10.1, operator ruling 2026-08-30). The push still does not
live in this Makefile: it is not undoable, and a convenience target for an
irreversible act is how the act happens by accident. **No image has ever been
pushed**, and none can be today: the release workflow carries the shape and not
the trigger, and it has no image job whatever the ruling says. The crate is a
different channel and a decided one — bl-006e adjudicated it and 0.0.1 is on
crates.io — so the two publications are no longer one question.

### The image-side disclosure gate

That registry ruling is **conditional**, and `make image-scan` is the
condition. It runs as the last step of `make image`, so no image exists on this
box that has not been read.

**It is a second gate and not a reuse of the first.** `make leak-scan` reads
the git INDEX; an image is built from inputs no commit has — the build context
as the engine actually receives it, the base image's layers, the package index,
and the image CONFIG. The list just below says the image carries no
certificate and no tool document; until this target, nothing had read a layer
to check that claim.

It reads three surfaces with the **same rule table** the commit gate uses
(`scripts/leak-rules.sh`, sourced and never copied):

- **The authored filesystem** — every file or symlink whose bytes differ from
  the pinned base image at that path. Both filesystems are exported and
  compared here rather than diffing layer digests: it needs no JSON parser,
  it works on docker as well as podman, and it is the finer answer, since a
  file the build rewrote to identical bytes is not authored content.
- **The distro floor is accounted for, not exempted.** The runtime layer runs
  `apk add`, which adds hundreds of files this repo did not write. apk's own
  ownership ledger says which package owns each one; a symlink resolving into
  that set is aliased distro content; everything else above the base is this
  repo's and is scanned. A path exemption would be an allowlist, and an
  allowlist is where a leak hides.
- **The image config** — every `Env`, `Label` and history entry. An `ENV` ships
  to everyone who pulls whether or not a file holds it, and build arguments
  echo into history.

The posture the commit gate already holds carries over unchanged. Findings
**locate** and never reprint (truncated to twelve characters). **Unreadable is
rejected, not skipped**: the one binary this build authors is `thrall`, and the
expected set is DERIVED from the Containerfile's `COPY --from=` destination
rather than typed into the scanner — any *other* authored file the rules cannot
read is a refusal. And **both directions**, because a scan that has stopped
matching passes everything forever: `make image-scan` first builds a scratch
image that layers a fabricated secret into a file, another into an `ENV`, and
an undeclared binary beside them, and requires all three findings, before
scanning the real image.

It is not part of `make check`, deliberately: `check` runs on boxes with no
container engine and must not depend on an artifact a build step produced.
It is the image's gate and it runs where the image is made.

What it cannot promise, stated rather than implied: it scans one image, on the
box that built it, before the push. It does not read what is already in a
registry, it cannot un-publish a digest, and whoever runs the build can bypass
it exactly as `--no-verify` bypasses the commit hook. That is the same
prevention-is-local split *The rules* records for the source gate, one artifact
over.

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
  dials its own channels again when they drop, but it never restarts its own
  process: a foot that cannot be a foot at all is an exit naming why, because
  restart policy belongs to this machine's own supervision, and putting a
  process-level retry loop in the image would take that decision away from the
  operator who has to live with it.
- **No bootstrap of any kind.** thrall mints nothing (REMOTE §1.4, DESIGN
  §3.3); an image that could provision itself would be exactly the flow that
  must never exist.

Every line of that list is a claim about bytes, and `make image-scan` is what
turns it from a promise into a check.

## The macOS artifact

    make mac-artifact        # -> dist/aarch64-apple-darwin/thrall

A foot runs wherever the operator's tools are, and some of those boxes are
macs. `make mac-artifact` cross-produces the `aarch64-apple-darwin` binary
**from the same Linux container line the image comes off** — the same
digest-pinned base, the same toolchain pin checked against
`rust-toolchain.toml`, the same `--locked` dependency answer the gate judged —
so the mac binary is reproducible from the tree rather than being whatever came
out of somebody's laptop that afternoon.

The product is a **file, not an image**. The build's last stage is `FROM
scratch` carrying one binary; the wrapper is a fixture, is never pushed, and is
deleted when the artifact has been lifted out. `make image-scan` therefore does
not apply to it and is not being skipped: the artifact's content is compiled
from the same tree `make leak-scan` reads, exactly as the Linux release binary
is.

### The toolchain is `zig cc`, and osxcross is refused

There were two ways to link a Mach-O binary on Linux, and the choice was made
on Apple's licence rather than on taste. osxcross drives **Apple's own SDK**,
which the *Xcode and Apple SDKs Agreement* forbids twice over — either clause
alone would settle it:

> **2.7** The grants set forth in this Agreement do not permit You to, and You
> agree not to, install, use or run the Apple Software or Apple Services on any
> non-Apple-branded computer or device, or to enable others to do so. … You
> agree not to rent, lease, lend, upload to or host on any website or server,
> sell, redistribute, or sublicense the Apple Software and Apple Services, in
> whole or in part, or to enable others to do so.

> **2.5** You may not alter the Apple Software or Services in any way in such
> copy, e.g., You are expressly prohibited from separately using the Apple SDKs
> or attempting to run any part of the Apple Software on non-Apple-branded
> hardware.

The first means the SDK may never sit in this repository nor in anything
published from it. The second means the usual escape — take the SDK path as a
build argument, keep it out of the tree, let the operator supply it — **does
not work either**, because the builder is not Apple-branded hardware. So this
repo does not hold the SDK at arm's length; it refuses the arm. A
`Containerfile` that accepted an SDK path would be inviting the operator into a
term they cannot satisfy on a Linux box.

`zig` acquires nothing from Apple: it ships one darwin stub of its own,
`lib/libc/darwin/libSystem.tbd`, in its own distribution and under its own
licence, and no Apple agreement is accepted anywhere on this path. It is
pinned by version **and** sha256; `cargo-zigbuild` (which filters the darwin
linker flags `zig cc` will not take) is pinned exactly and installed
`--locked`.

**And yes, it is a C toolchain, deliberately.** `deny.toml` bans
`openssl-sys`, `native-tls` and `aws-lc-sys` to stop a C toolchain arriving
*implicitly*, through a dependency edge nobody reviewed. This one arrives
explicitly, in a file that argues for it, in a build stage that is discarded —
and `Containerfile` already installs `musl-dev` for `ring` on the same terms.
The posture is against the accident, not against the compiler.

### The limit that comes with it

zig ships libSystem and **no framework stubs at all** — no CoreFoundation, no
AppKit, no OpenGL, no `libobjc`. A crate graph that links only libSystem
crosses cleanly; one that links any Apple framework fails at the link step with
*"unable to find framework"*, and there is no lawful way to supply the
frameworks on a Linux builder.

**thrall links none of them.** Its dependency set is `rustls`/`ring` and
`serde_json` over std, and the verified artifact loads exactly three system
libraries, all under `/usr/lib`. That is not luck — it is the same
single-binary discipline the approved dependency set in `Cargo.toml` is written
to keep, and it is what makes the foot the component whose mac build a Linux
container can honestly produce.

### What is proven, and what is not

There is no mac on the build box, so **the artifact is never executed**. A
green build is not evidence: a wrong architecture, a dependency on a dylib no
stock mac carries, and a binary macOS would refuse to start all look identical
to a successful `cargo build`. `scripts/mac-verify.sh` reads the produced file
instead, on any platform, with no Apple tooling:

- **Proven** — 64-bit Mach-O, `arm64`, an executable (not a dylib); platform
  macOS with the minimum-OS and SDK versions it declares; every dynamic library
  it will ask for at load time, each of which must be a stock `/usr/lib` or
  `/System/Library` path; and that a code signature is present at all.
- **Not proven** — that it runs. It has the shape of a working mac binary and
  has not been observed to be one.

It runs **both directions**, the discipline `leak-scan` and `rules-audit`
already hold here: five fabricated malformed inputs must be refused before the
real artifact is read, because a checker that has quietly stopped checking
passes everything forever.

Two properties are worth knowing before an artifact is handed to anyone:

- **The minimum macOS version is the pinned zig's, not a setting.** `rustc`
  asks for one and this zig stamps its own; `cargo-zigbuild`'s
  `aarch64-apple-darwin.<version>` target syntax does not survive this zig
  either. So the floor is a property of the pinned pair — read it off the
  artifact, where `mac-verify` prints it, and never from a document.
- **The signature is ad-hoc, and that is not notarization.** An arm64 mac
  refuses to start an unsigned binary; the cross-linker's ad-hoc signature
  satisfies that and nothing more. A copy that arrives over a network carries a
  quarantine attribute, and clearing it — or replacing the signature with a
  real one — is an act on a mac, by the operator, and it is outside what this
  line can do.

Only `aarch64` is produced, because that is what was asked for. `x86_64-apple-darwin`
would cross on exactly the same terms (the libSystem-only rule is about the
crate graph, not the architecture) and is not built because nothing wants it.

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
