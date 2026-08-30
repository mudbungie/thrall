.PHONY: all build release test coverage lint fmt fmt-check check ci \
        line-cap leak-scan rules-audit deny install-hooks install uninstall \
        image image-scan clean

# The build authority. Every gate step has ONE home here, and the pre-commit
# hook calls the same targets — so the hook, a hand-run `make check` and any
# future CI cannot drift into three different definitions of "green".

# Install location for `make install`. Defaults to the XDG-ish user-local
# convention; override for system-wide installs or packaging:
#   make install INSTALL_PREFIX=/usr/local
INSTALL_PREFIX ?= $(HOME)/.local
INSTALL_BIN    := $(INSTALL_PREFIX)/bin

# Build output root. Exported so every cargo invocation below honours an
# override.
CARGO_TARGET_DIR ?= target
export CARGO_TARGET_DIR

all: check

build:
	cargo build

release:
	cargo build --release

test:
	cargo test

TARPAULIN_PIN := 0.35.2

# The 100% coverage floor. The pin is checked before the run rather than
# after: a 0.35.4+ tarpaulin silently drops inline `#[cfg(test)] mod tests`
# files from the coverable denominator, so an unpinned run reports a weaker
# floor as a pass. See tarpaulin.toml.
coverage:
	@have=$$(cargo tarpaulin --version 2>/dev/null | awk '{print $$NF}'); \
	if [ "$$have" != "$(TARPAULIN_PIN)" ]; then \
	  echo "tarpaulin $(TARPAULIN_PIN) required (have: $${have:-none}); see tarpaulin.toml" >&2; \
	  echo "  cargo install cargo-tarpaulin --version $(TARPAULIN_PIN) --locked" >&2; \
	  exit 1; \
	fi
	cargo tarpaulin --fail-under 100 --skip-clean --engine llvm --out Stdout

# The complete static gate: the line cap + clippy (which reads Cargo.toml
# [lints]) + the ast-grep rules audit + the supply-chain audit. Every tool is
# pinned so the gate is reproducible — ast-grep 0.44.1 (sgconfig.yml),
# cargo-deny 0.20.2 (deny.toml), toolchain 1.95.0 (rust-toolchain.toml).
#
# `line-cap` goes first because it is milliseconds, and `leak-scan` second
# because it is seconds: a structural violation and a disclosure should both
# fail before the minute-scale tools start.
lint:
	$(MAKE) line-cap
	$(MAKE) leak-scan
	cargo clippy --all-targets -- -D warnings
	$(MAKE) rules-audit
	$(MAKE) deny

# The 300-line hard cap on every tracked source file, inline tests included.
# Docs and config are exempt. THIS TARGET IS THE ONE DEFINITION of the cap and
# of what counts as a source file; the pre-commit hook calls it and restates
# nothing.
#
# It scans the WHOLE TREE, not the staged diff. A hook that walked only the
# staged files would make the cap a sampling rather than an invariant: a file
# that crossed the cap and was never touched again would never be looked at
# again. `git ls-files` reads the INDEX, so a staged addition is covered before
# it is ever committed and a staged deletion is already gone.
#
# The cap is a variable, so the same target answers the design-time question:
# `make line-cap LINE_CAP=199` lists the >=200 pre-split band. That stays a
# hand-run view and never a gate — 300 is a WALL, not a target. A file resting
# ON the wall inverts the rule, firing on whoever touches it next, at the
# moment they are finishing something else, when the cheapest way out is
# exactly the line-shaving the rule forbids. Over the cap? Split along a real
# seam and record it in docs/DESIGN.md; never shave lines to duck the limit.
#
# The empty-set guard is this target's own negative check, the same
# two-direction discipline `rules-audit` holds: a broken pattern or a wrong
# working directory would otherwise enumerate nothing and pass silently, which
# is the exact failure this target exists to end.
LINE_CAP := 300
LINE_CAP_EXEMPT := \.(md|txt|toml|yaml|yml|json|lock)$$|(^|/)(Makefile|LICENSE|\.gitignore|\.githooks/)

line-cap:
	@files=$$(git ls-files | grep -Ev '$(LINE_CAP_EXEMPT)' || true); \
	n=$$(printf '%s\n' "$$files" | grep -c . || true); \
	over=$$(printf '%s\n' "$$files" | while IFS= read -r f; do \
	    { [ -n "$$f" ] && [ -f "$$f" ]; } || continue; \
	    c=$$(wc -l < "$$f"); \
	    [ "$$c" -gt $(LINE_CAP) ] && printf '  %s: %s lines\n' "$$f" "$$c"; \
	    true; \
	  done); \
	if [ "$$n" -eq 0 ]; then \
	  echo "line-cap: enumerated 0 source files — the scan is broken, not the tree" >&2; \
	  exit 1; \
	fi; \
	if [ -n "$$over" ]; then \
	  echo "error: source files over the $(LINE_CAP)-line cap:" >&2; \
	  printf '%s\n' "$$over" >&2; \
	  echo "       split along a real seam (docs/DESIGN.md) — do not shave lines." >&2; \
	  exit 1; \
	fi; \
	echo "line-cap: $$n source files, all within $(LINE_CAP) lines"

# The disclosure gate (bl-e878, ported from yog): no credential, routable
# address, MAC, home path, email, pasted dialogue, agent-session artifact,
# credential-shaped path or unreadable blob in the tree.
# `scripts/leak-rules.sh` is the ONE definition of what counts,
# `scripts/leak-scan.sh` runs it, and this target is the door — neither
# restates the other.
#
# BOTH DIRECTIONS in one target, the same discipline `rules-audit` holds: the
# self-test runs FIRST, and it is the stronger of the two checks. Every rule
# owns a fixture in which every non-comment line must be flagged BY THAT RULE
# and must carry the `notreal` marker, plus `clean.txt`/`clean-paths.txt` of
# near-misses that must NOT be flagged. A leak gate does not die by being
# wrong; it dies by silently matching nothing after a pattern is edited, and
# then passing everything forever — and a gate that cries wolf gets bypassed,
# which is the same death by the other road.
#
# It reads INDEX BLOBS, not the worktree: `git checkout-index` materializes the
# index into a scratch tree and the scan reads that, so the bytes scanned are
# the bytes committed. A leak that is `git add`ed and then overwritten with a
# clean copy on disk is still caught.
#
# The COMMIT MESSAGE is not in any tree, so no pre-commit step can see it;
# `.githooks/commit-msg` runs this same scanner over it. `make install-hooks`
# seats both.
leak-scan:
	@scripts/leak-scan.sh --self-test
	@scripts/leak-scan.sh

# Static audit of every ast-grep rule (rules/, pinned ast-grep 0.44.1 — see
# sgconfig.yml). BOTH DIRECTIONS: `src` must be clean, AND every rule must
# still flag its deliberate violation in rules/fixtures, so a rule whose
# pattern silently stopped matching anything cannot pass as green forever.
#
# PER RULE, NOT PER DIRECTORY (bl-1827). This used to ask only whether
# `rules/fixtures` was flagged by SOMETHING, which nine live rules answer for a
# tenth dead one forever. It now runs each rule ALONE — `--filter` on the `id`
# read out of the rule's own file — and fails the rule that flags nothing. Two
# things follow, and the second is why the change was worth making:
#
#   - a rule that stops matching is named, individually, on the run it breaks;
#   - a rule with NOTHING TO MATCH IN `src` is measurable at all. The four
#     confinement rules are exactly that: thrall has no `unsafe`, no lock and
#     no child process, so `ast-grep scan src` is silent about them whether
#     they work or not, and this loop is the only thing that says they do.
#
# The id is read from the file rather than kept in a list here, so a new rule
# cannot be added to a stale list — and a new rule with no fixture fails on the
# run that adds it. The empty-set guard is the same discipline `line-cap`
# holds: enumerating no rules at all is a broken audit, not a clean tree.
rules-audit:
	ast-grep scan src
	@n=0; for r in rules/*.yml; do \
	  id=$$(sed -n 's/^id:[[:space:]]*//p' "$$r" | head -1); \
	  if [ -z "$$id" ]; then \
	    echo "rules-audit: $$r declares no id" >&2; exit 1; \
	  fi; \
	  n=$$((n + 1)); \
	  if ast-grep scan --filter "^$$id$$" rules/fixtures >/dev/null 2>&1; then \
	    echo "rules-audit: [$$id] flagged NOTHING in rules/fixtures — the rule has" >&2; \
	    echo "             regressed, or it was added without a fixture. Fix the rule" >&2; \
	    echo "             or write the violation; never delete the check." >&2; \
	    exit 1; \
	  fi; \
	done; \
	if [ "$$n" -eq 0 ]; then \
	  echo "rules-audit: enumerated 0 rules — the audit is broken, not the tree" >&2; \
	  exit 1; \
	fi; \
	echo "rules-audit: src clean; all $$n rules flagged their fixture"

# Supply-chain audit (cargo-deny 0.20.2 — see deny.toml): licenses, advisories,
# the TLS-stack bans, and registry-only sources.
deny:
	cargo deny check

fmt:
	cargo fmt

fmt-check:
	cargo fmt --check

# The complete gate, and the exact target any CI runs. Cheap steps first.
check: fmt-check lint coverage

ci: check

# Arm this clone's git hooks: one symlink per file in .githooks/, seated in the
# repo's own hooks directory. Symlinks, not copies, so an updated hook is live
# without a re-run.
#
# NOT `core.hooksPath`, which a machine may set globally to a chain hook; a
# per-repo override would silence that machine-wide hook for this repo, while
# seating the links where git already looks keeps both.
#
# Refused from a linked worktree: `bl claim` deletes those, and links pointing
# into one would rot the moment the ball closed.
install-hooks:
	@top=$$(git rev-parse --path-format=absolute --show-toplevel) && \
	common=$$(git rev-parse --path-format=absolute --git-common-dir) && \
	if [ "$$common" != "$$top/.git" ]; then \
	  echo "install-hooks: run this in the main checkout, not a linked worktree" >&2; \
	  exit 1; \
	fi; \
	mkdir -p "$$common/hooks"; \
	for h in .githooks/*; do \
	  ln -sfn "$$top/$$h" "$$common/hooks/$${h#.githooks/}"; \
	done; \
	echo "hooks: seated $$(ls .githooks | tr '\n' ' ')in $$common/hooks"

# A foot is a program that runs on a machine somebody else administers, so the
# binary gets rename(2) atomicity: write a temp name in the SAME directory,
# then `mv -f` it into place. A supervisor restarting mid-install then sees
# whole-old or whole-new, never the ENOENT window install(1) opens between its
# unlink and its write.
install: release
	@mkdir -p "$(INSTALL_BIN)"
	@install -m 0755 $(CARGO_TARGET_DIR)/release/thrall "$(INSTALL_BIN)/.thrall.tmp" && \
	  mv -f "$(INSTALL_BIN)/.thrall.tmp" "$(INSTALL_BIN)/thrall"
	@echo "installed $(INSTALL_BIN)/thrall"

uninstall:
	@rm -f "$(INSTALL_BIN)/thrall"
	@echo "removed $(INSTALL_BIN)/thrall"

# The OCI image — the unit of install for a box that takes containers rather
# than binaries. `Containerfile` is the whole of what it builds and states why
# each layer is what it is.
#
# The version is READ FROM Cargo.toml and never typed here: the crate version
# has one home, and a tag typed into a Makefile is that fact stored twice. Both
# `:<version>` and `:latest` are applied to the same build.
#
# Podman or docker, whichever the box has, podman first — it needs no daemon
# and no group membership, which is the difference between "the operator can
# build this" and "the operator can build this after an admin says yes".
# Override with `make image ENGINE=docker`.
#
# IT PUSHES NOTHING, and there is no `push` target to forget to guard. The
# registry is now named — `ghcr.io/mudbungie/thrall`, one package per repo,
# pushed only from that repo's release workflow at tag time (yog DESIGN §10.1,
# operator ruling 2026-08-30) — and the push still does not live here. A push
# is not undoable: a tag can move, but the bytes anyone pulled are theirs. What
# publishes is the version tag and the manifest digest, both immutable, and
# never a moving `latest`; the `:latest` applied below is LOCAL, a convenience
# on one box nobody else can pull.
#
# thrall has no remote and no release workflow yet (bl-006e), so nothing here
# can push today whatever the ruling says. The gate below still lands first,
# for the reason the confinement rules landed ahead of the surfaces they govern
# (DESIGN §5.2): a rule installed after the first site is a rule that has to be
# argued with.
IMAGE_NAME    ?= thrall
IMAGE_VERSION := $(shell sed -n '/^\[package\]/,/^\[/{s/^version *= *"\([^"]*\)".*/\1/p;}' Cargo.toml)
IMAGE_TAG     := $(IMAGE_NAME):$(IMAGE_VERSION)
ENGINE ?= $(shell command -v podman 2>/dev/null || command -v docker 2>/dev/null)

image:
	@test -n "$(ENGINE)" || { echo "image: no podman and no docker on PATH" >&2; exit 1; }
	@test -n "$(IMAGE_VERSION)" || { echo "image: no version in Cargo.toml" >&2; exit 1; }
	@echo "image: $(notdir $(ENGINE)) build -> $(IMAGE_TAG)"
	@$(ENGINE) build -f Containerfile \
	  -t "$(IMAGE_TAG)" -t "$(IMAGE_NAME):latest" .
	@$(ENGINE) image inspect "$(IMAGE_TAG)" \
	  --format 'image: {{.Id}} {{.Size}} bytes'
	@$(MAKE) --no-print-directory image-scan

# The image-side disclosure gate — yog DESIGN §10.1's condition on the registry
# ruling, and the check nothing in this repo previously performed. `leak-scan`
# reads the git INDEX; an image is built from inputs no commit has — the build
# context as the engine receives it, the base layers, the package index, and
# the image CONFIG. A foot's image is the one that matters most: the
# Containerfile promises no certificate and no `tools.json` in any layer, and
# until this target nothing read the layers to check.
#
# It is a step of `image` and not a target beside it, for the reason the
# pre-commit hook is not a target beside `commit`: a gate a person has to
# remember to run is not a gate. Run it alone to re-judge an image already
# built. `scripts/image-scan.sh` states what it scans and how it isolates the
# authored content; this target only decides which tag and runs BOTH
# directions — the planted-secret self-test first, because a scan that has
# stopped matching passes everything forever, then the real image.
#
# NOT part of `check`. `check` must run on a box with no container engine and
# must not depend on an artifact a build step produced; this needs both. It is
# the image's gate, and it runs where the image is made.
image-scan:
	@test -n "$(ENGINE)" || { echo "image-scan: no podman and no docker on PATH" >&2; exit 1; }
	@test -n "$(IMAGE_VERSION)" || { echo "image-scan: no version in Cargo.toml" >&2; exit 1; }
	@ENGINE=$(ENGINE) scripts/image-scan.sh --self-test "$(IMAGE_TAG)"
	@ENGINE=$(ENGINE) scripts/image-scan.sh "$(IMAGE_TAG)"

# There is deliberately NO `publish` target. `Cargo.toml` carries
# `publish = false`, the registry name is held by a placeholder, and whether
# thrall ever ships is an operator decision (bl-006e). A convenience target for
# an irreversible act is how the act happens by accident.

clean:
	cargo clean
