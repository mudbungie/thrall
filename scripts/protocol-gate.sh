#!/usr/bin/env bash
# protocol-gate.sh — this foot does not publish a wire protocol version that no
# published engine speaks (bl-635b; yog bl-bca2 is the other direction).
#
#   scripts/protocol-gate.sh hello        this repo's hello file, one path
#   scripts/protocol-gate.sh read FILE    the PROTOCOL integer that file states
#   scripts/protocol-gate.sh judge PUBLISHED CANDIDATE
#   scripts/protocol-gate.sh --self-test  both directions, no network
#
# WHY THIS EXISTS. yog mints the wire protocol version; this repository, the
# seat and the phone each VENDOR a copy of the constant, and the wire is
# fail-closed on a mismatch with no negotiation (yog `docs/REMOTE.md` §3). The
# skew that opens is therefore two-directional and each direction is gated in
# the repository that can decide it:
#
#   1. yog publishing a bump AHEAD of its consumers. Held at yog's release pull
#      request until this repository's `main` — and lernie's, and
#      yog-android's — carries the new number. That is yog bl-bca2.
#   2. THIS repository publishing a bump ahead of the engine. thrall 0.0.15
#      published PROTOCOL 16 while the newest published yog, 0.0.49, spoke 15:
#      a foot published ahead of every engine a user can install, composing
#      with nothing, by exactly the road the other direction took.
#
# THE ORDERING THE TWO GATES PRODUCE, and the one thing to remember when
# bumping: **the consumers' mains carry the number first, then yog publishes,
# then the consumers publish.** Gate 1 enforces the first arrow, gate 2 the
# second. Landing the constant on `main` is never held by either — it is what
# gate 1 waits for.
#
# STRICTLY GREATER, NOT DIFFERENT. A thrall BEHIND the published engine is
# direction 1's defect, and its release is the fix; holding it would hold the
# remedy shut.
#
# THIS FILE IS PURE LOGIC AND READS NO NETWORK, which is what makes the rule
# testable: `.github/workflows/release-plz.yml`'s `merge-release-pr` job
# fetches the two `hello.rs` files — yog's at its newest `v<x.y.z>` tag, and
# this repository's at the release pull request's head — and hands this script
# the paths.

set -euo pipefail

# The wire constant's one home here. `src/corpus.rs` keeps a second copy that
# this crate's own tests hold equal to it; the one that decides a handshake is
# the one read.
HELLO='src/channel/hello.rs'

# The declaration, anchored at the start of a line so a doc comment quoting it
# — `src/corpus.rs` quotes the line verbatim — is not mistaken for it. `pub`,
# `pub(crate)` and a bare `const` all read, because the four repositories that
# vendor this constant do not spell its visibility the same way.
DECL='^[[:space:]]*(pub[[:space:]]*(\([^)]*\))?[[:space:]]+)?const[[:space:]]+PROTOCOL[[:space:]]*:[[:space:]]*u32[[:space:]]*=[[:space:]]*([0-9]+)[[:space:]]*;.*$'

# The integer FILE states, or nothing and a non-zero status. No pipe: `sed |
# head` would kill the writer with SIGPIPE and `pipefail` would then report the
# read as failed exactly when it succeeded.
protocol_of() {
  local file=$1 hits
  [ -r "$file" ] || return 1
  hits=$(sed -nE "s/$DECL/\\3/p" "$file")
  [ -n "$hits" ] || return 1
  printf '%s\n' "${hits%%$'\n'*}"
}

# The verdict, as one line on stdout. Exit 0 merges, 3 holds. Every unreadable
# input holds: a gate that could not read its inputs has not answered, and a
# hold is undone by the next run while a publish is undone by nothing.
judge() {
  local published=$1 candidate=$2 pub cand
  if ! cand=$(protocol_of "$candidate"); then
    echo "hold: no PROTOCOL declaration in this release's $candidate"
    return 3
  fi
  if ! pub=$(protocol_of "$published"); then
    echo "hold: the published engine's PROTOCOL could not be read"
    return 3
  fi
  if [ "$cand" -gt "$pub" ]; then
    echo "hold: this release speaks PROTOCOL $cand and the published engine speaks $pub"
    return 3
  fi
  echo "merge: PROTOCOL $cand, and the published engine speaks $pub"
}

# --- the self-test ----------------------------------------------------------
# Both directions, because the workflow that spends this cannot run locally and
# a check that has stopped matching passes everything forever.
fixture() {
  printf '//! /// pub const PROTOCOL: u32 = 999;\n%s\n' "$2" >"$1"
}

expect() {
  local want_rc=$1 want=$2 got rc=0
  shift 2
  got=$(judge "$@") || rc=$?
  if [ "$rc" != "$want_rc" ] || [ "$got" != "$want" ]; then
    echo "protocol-gate self-test: judge $*" >&2
    echo "  expected rc $want_rc: $want" >&2
    echo "  answered rc $rc: $got" >&2
    exit 1
  fi
}

self_test() {
  local d
  d=$(mktemp -d)
  # shellcheck disable=SC2064
  trap "rm -rf '$d'" EXIT

  fixture "$d/p15" 'pub const PROTOCOL: u32 = 15;'
  fixture "$d/p16" '    pub(crate) const PROTOCOL: u32 = 16;'
  fixture "$d/bare15" 'const PROTOCOL: u32 = 15;'
  fixture "$d/quoted" '/// reads `pub const PROTOCOL: u32 = 15`, copied below.'

  # The 2026-09-06 tree: this repo at 16, the published engine at 15.
  expect 3 'hold: this release speaks PROTOCOL 16 and the published engine speaks 15' \
    "$d/p15" "$d/p16"
  # The same release once yog has published 16.
  expect 0 'merge: PROTOCOL 16, and the published engine speaks 16' \
    "$d/p16" "$d/p16"
  # Level, however each end spells the constant.
  expect 0 'merge: PROTOCOL 15, and the published engine speaks 15' \
    "$d/bare15" "$d/p15"
  # BEHIND the engine is direction 1's defect and this release is its fix:
  # strictly greater, so it publishes.
  expect 0 'merge: PROTOCOL 15, and the published engine speaks 16' \
    "$d/p16" "$d/p15"
  # Fail closed on each unreadable input, and never merge on one. The quoted
  # fixture also proves the anchor: a doc comment is not a declaration.
  expect 3 "hold: no PROTOCOL declaration in this release's $d/quoted" \
    "$d/p15" "$d/quoted"
  expect 3 'hold: the published engine'"'"'s PROTOCOL could not be read' \
    "$d/quoted" "$d/p16"
  expect 3 'hold: the published engine'"'"'s PROTOCOL could not be read' \
    "$d/absent" "$d/p16"

  if [ ! -r "$HELLO" ] || ! protocol_of "$HELLO" >/dev/null; then
    echo "protocol-gate self-test: $HELLO states no PROTOCOL — the path has moved" >&2
    exit 1
  fi
  echo "protocol-gate: self-test OK — 7 verdicts both ways, and $HELLO still states one" >&2
}

case ${1:-} in
--self-test) self_test ;;
hello) printf '%s\n' "$HELLO" ;;
read) [ "$#" = 2 ] || { echo "usage: protocol-gate.sh read FILE" >&2; exit 2; }
      protocol_of "$2" || { echo "protocol-gate: no PROTOCOL declaration in $2" >&2; exit 1; } ;;
judge) [ "$#" = 3 ] || { echo "usage: protocol-gate.sh judge PUBLISHED CANDIDATE" >&2; exit 2; }
       judge "$2" "$3" ;;
*) echo "usage: protocol-gate.sh {hello|read FILE|judge PUBLISHED CANDIDATE|--self-test}" >&2; exit 2 ;;
esac
