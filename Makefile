.PHONY: all build release test coverage lint fmt fmt-check check ci \
        line-cap rules-audit deny install-hooks install uninstall clean

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
# `line-cap` goes first because it is milliseconds: a structural violation
# should fail before the minute-scale tools start.
#
# NOT YET IN THIS LADDER, and named so the absence is a decision rather than an
# oversight: the disclosure gate (bl-e878). thrall ships no `scripts/leak-scan.sh`,
# which is exactly the signal the machine-level store gate keys opt-in on — so
# thrall is currently NOT gated on what its task bodies publish.
lint:
	$(MAKE) line-cap
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

# Static audit of every ast-grep rule (rules/, pinned ast-grep 0.44.1 — see
# sgconfig.yml). BOTH DIRECTIONS: `src` must be clean, AND every deliberate
# violation in rules/fixtures must fire, so a rule whose pattern silently
# stopped matching anything cannot pass as green forever.
rules-audit:
	ast-grep scan src
	@if ast-grep scan rules/fixtures >/dev/null 2>&1; then \
	  echo "rules-audit: rules/fixtures was NOT flagged — a rule has regressed" >&2; \
	  exit 1; \
	fi
	@echo "rules-audit: src clean; fixtures flagged (all rules live)"

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

# There is deliberately NO `publish` target. `Cargo.toml` carries
# `publish = false`, the registry name is held by a placeholder, and whether
# thrall ever ships is an operator decision (bl-006e). A convenience target for
# an irreversible act is how the act happens by accident.

clean:
	cargo clean
