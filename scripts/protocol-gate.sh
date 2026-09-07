#!/usr/bin/env bash
# protocol-gate.sh — this foot does not publish a wire protocol version that no
# published engine speaks (bl-635b; yog bl-bca2 is the other direction).
#
#   scripts/protocol-gate.sh file         the path every repo states it at
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
# fetches the two `PROTOCOL` files — yog's at its newest `v<x.y.z>` tag, and
# this repository's at the release pull request's head — and hands this script
# the paths.
#
# THE NUMBER IS A FILE, NOT A DECLARATION (bl-c618, yog bl-3e57). Both reads
# here are a fetch of one path out of a tree this gate does not build, and a
# Rust path is not a stable address for that: yog split `src/wire/hello.rs`
# into `src/wire/hello/version.rs` and left the old path re-exporting, which a
# build cannot notice and a regex reads as no declaration. This gate went on
# fetching the old path, read the engine's number as ABSENT, and held — the
# fail-closed answer, and the wrong one, because the input was readable at
# another address. The next engine publish would have made that hold permanent
# rather than clearing it. So the number now has one file-shaped home per
# repository: a top-level `PROTOCOL` file, one line, the integer, compiled into
# the constant by `build.rs`. There is NO Rust path in this file.

set -euo pipefail

# The path, at the root of this repository and of the engine's alike. One
# constant for both reads, because one address is the whole point: a
# per-repository path is a per-repository way to rot.
FILE='PROTOCOL'

# The integer FILE states, or nothing and a non-zero status. The whole file is
# the number: any second line, any word, any punctuation is not a PROTOCOL file
# and reads as unstated rather than as a number found inside something else.
# Command substitution strips the trailing newline, and a newline is not a
# digit, so the one `case` rejects an empty file and a multi-line one alike.
protocol_of() {
  local file=$1 stated
  [ -r "$file" ] || return 1
  stated=$(<"$file")
  case $stated in
  '' | *[!0-9]*) return 1 ;;
  esac
  printf '%s\n' "$stated"
}

# The verdict, as one line on stdout. Exit 0 merges, 3 holds. Every unreadable
# input holds: a gate that could not read its inputs has not answered, and a
# hold is undone by the next run while a publish is undone by nothing.
judge() {
  local published=$1 candidate=$2 pub cand
  if ! cand=$(protocol_of "$candidate"); then
    echo "hold: this release's $candidate states no PROTOCOL integer"
    return 3
  fi
  if ! pub=$(protocol_of "$published"); then
    # This repository's expectation, named as this repository's — the engine's
    # file is not the thing that failed. A yog release predating its own
    # PROTOCOL file states nothing at this path, and so does a fetch that never
    # answered.
    echo "hold: the published engine states no PROTOCOL integer at its repo root, which is where this gate reads it"
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
# A tree's `PROTOCOL` file, written byte for byte — `%b` so a case can state
# its own line endings, which is half of what these fixtures are for.
fixture() {
  printf '%b' "$2" >"$1"
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

  fixture "$d/n15" '15\n'
  fixture "$d/n16" '16\n'
  # A file is the number and nothing else, but the number may be spelled with
  # the whitespace an editor leaves.
  fixture "$d/tight16" '16'
  fixture "$d/loose15" '15\n\n'
  # What the number's home used to be. A Rust declaration is now exactly as
  # unreadable as prose, which is the point: this file names no Rust path, so a
  # tree that still keeps its number in a module states nothing.
  fixture "$d/decl" 'pub const PROTOCOL: u32 = 15;\n'

  # The 2026-09-06 tree: this repo at 16, the published engine at 15.
  expect 3 'hold: this release speaks PROTOCOL 16 and the published engine speaks 15' \
    "$d/n15" "$d/n16"
  # The same release once yog has published 16.
  expect 0 'merge: PROTOCOL 16, and the published engine speaks 16' \
    "$d/n16" "$d/n16"
  # Level, however each end terminates its file.
  expect 0 'merge: PROTOCOL 15, and the published engine speaks 15' \
    "$d/loose15" "$d/n15"
  # BEHIND the engine is direction 1's defect and this release is its fix:
  # strictly greater, so it publishes.
  expect 0 'merge: PROTOCOL 15, and the published engine speaks 16' \
    "$d/tight16" "$d/n15"
  # Fail closed on each unreadable input, and never merge on one. The `decl`
  # fixture is the defect this shape ended: the number in a Rust file is read
  # by nothing here.
  expect 3 "hold: this release's $d/decl states no PROTOCOL integer" \
    "$d/n15" "$d/decl"
  expect 3 'hold: the published engine states no PROTOCOL integer at its repo root, which is where this gate reads it' \
    "$d/decl" "$d/n16"
  expect 3 'hold: the published engine states no PROTOCOL integer at its repo root, which is where this gate reads it' \
    "$d/absent" "$d/n16"

  # This repository's own file, read as the workflow will read it: the gate is
  # worthless if the tree it ships in does not state the number where it looks.
  if [ ! -r "$FILE" ] || ! protocol_of "$FILE" >/dev/null; then
    echo "protocol-gate self-test: $FILE does not state one integer — the number's home has moved" >&2
    exit 1
  fi
  echo "protocol-gate: self-test OK — 7 verdicts both ways, and $FILE states one" >&2
}

case ${1:-} in
--self-test) self_test ;;
file) printf '%s\n' "$FILE" ;;
read) [ "$#" = 2 ] || { echo "usage: protocol-gate.sh read FILE" >&2; exit 2; }
      protocol_of "$2" || { echo "protocol-gate: $2 states no PROTOCOL integer" >&2; exit 1; } ;;
judge) [ "$#" = 3 ] || { echo "usage: protocol-gate.sh judge PUBLISHED CANDIDATE" >&2; exit 2; }
       judge "$2" "$3" ;;
*) echo "usage: protocol-gate.sh {file|read FILE|judge PUBLISHED CANDIDATE|--self-test}" >&2; exit 2 ;;
esac
