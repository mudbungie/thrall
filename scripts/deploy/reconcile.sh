#!/bin/sh
# **Unattended foot CD** (bl-6c98) — the timer-driven half of `local.sh`'s
# deployment. `local.sh` is a human at a keyboard; this runs on the box with
# nobody there.
#
#   reconcile.sh                # from thrall-reconcile.service, hourly
#   reconcile.sh --self-test    # checks the decision; touches no machine
#
# Operator ruling 2026-09-05: every device should run effectively full CD — any
# new publication should result in an upgrade of the running versions.
#
# TWO RECONCILIATIONS, NOT ONE PROCEDURE. They are separated because they have
# different safety conditions and must be free to happen at different times:
#
#   1. the INSTALLED binary against the registry's newest live version
#   2. the RUNNING foot against the installed binary
#
# (1) is always safe while the foot runs: `cargo install` replaces the file by
# rename, so the running process keeps its own inode and never sees a
# half-written binary. (2) is NOT always safe — a restart kills whatever
# invocation is executing — and that is the whole reason this file exists rather
# than a `cargo install && systemctl restart` line.
#
# **What a restart destroys is an invocation mid-flight**, and the damage is not
# symmetrical with the engine's. A killed turn on the engine costs the spend
# that bought it; a killed invocation here costs a command that already ran
# HALF. Its side effects on this box stand, the capture never comes back, and
# the engine learns only that its foot went away. Nothing settles that the way
# litany settles an unanswered tool window, because the act was not a message.
# So the deferral here is if anything more load-bearing than the engine's.
#
# **The idle read is the unit's own cgroup, and for a NATIVE unit that is
# honest.** thrall runs every tool as a child process (`src/spawn.rs`), and its
# own concurrency inside one process is threads, so an idle foot is exactly one
# process in its cgroup and an invocation in flight is a child beside it. That
# needs no thrall-side API, no new wire act and no field on the protocol — the
# fact is already there to read, and thrall's whole wire surface is three acts
# by design (README), none of which is "are you busy".
#
# The objection that retired this predicate on yog's engine does not reach here.
# There it was read off a CONTAINER unit's cgroup, which holds `docker run` — a
# client process — so the count answered about the wrong program. A native unit's
# cgroup holds the program.
#
# Nothing here is stored. "Which version is installed" and "which is running"
# are the two binaries' own answers, and "is it busy" is the cgroup's. There is no state file to drift, and nothing to reconcile if
# somebody installs or restarts by hand.
#
# Exit 0 whenever the box is in a lawful state, including "newer version
# installed, restart deferred" — a deferral is a correct steady state, the timer
# is the retry cadence, and a unit that reddened on one would cry wolf through
# every long invocation.
set -eu

CRATE=thrall
UNIT=thrall.service
# The install root, and it is not a preference: it is the Makefile's
# `INSTALL_PREFIX` default and the path `thrall.service` execs. Installing to
# cargo's default root instead would leave the new build where the unit never
# looks, and the CD would be silently dead — a timer updating a binary nothing
# runs, with no error anywhere.
ROOT="$HOME/.local"
BIN="$ROOT/bin/$CRATE"
# The sparse index — the same source `cargo install` resolves from, so the two
# can never disagree about what exists. Names of four characters or more live
# under `<first two>/<next two>/<name>`.
INDEX_URL="https://index.crates.io/th/ra/$CRATE"

say() { printf '%s: %s\n' "$CRATE-reconcile" "$*"; }
die() { say "$*" >&2; exit 1; }

# ------------------------------------------------------------- the decision
#
# ONE RULE: the foot should be running the newest live version, unless an
# invocation is executing or the operator stopped it. Every case below falls out
# of that sentence. It is a pure function of four facts so the branches that
# matter can be tested without a box, a release or an engine (`--self-test`).
# Prints exactly one of `current`, `defer`, `restart`.
#
# WHY `failed` IS ITS OWN ARM. A unit that could not start has no invocation to
# protect, so nothing has to be deferred for — but neither is there any point
# retrying the version that just failed. Trying a version it has NOT run is the
# one useful act, and together with the yank lever below that closes the loop: a
# release that cannot be a foot is recovered by yanking it, with nobody logging
# in. Without this arm a bad release strands the box until a human comes.
#
# `unknown` for any input defers: a fact we could not read is never grounds for
# killing work we cannot see.
decide() {
    _state=$1 _changed=$2 _pending=$3 _idle=$4
    if [ "$_state" = failed ]; then
        if [ "$_changed" = yes ]; then echo restart; else echo defer; fi
        return
    fi
    # Stopped on purpose, or mid-transition: not ours to move.
    if [ "$_state" != active ]; then echo current; return; fi
    # **Answered before the unknown guard, and the order is load-bearing.** The
    # idle question is only asked when there is something to defer FOR, so on a
    # box that is already current `idle` is `unknown` by construction — and an
    # unknown-first guard would answer `defer` on every tick of the steady
    # state, reporting an unreadable fact about a box with nothing to do.
    [ "$_pending" != no ] || { echo current; return; }
    case "$_pending$_idle" in
        *unknown*) echo defer; return ;;
    esac
    [ "$_idle" = yes ] || { echo defer; return; }
    echo restart
}

self_test() {
    _fail=0 _cases=0
    # state changed pending idle -> expected
    for _case in \
        'active   no  no      yes     current' \
        'active   no  no      no      current' \
        'active   no  no      unknown current' \
        'active   no  yes     yes     restart' \
        'active   yes yes     yes     restart' \
        'active   no  yes     no      defer' \
        'active   no  unknown yes     defer' \
        'active   no  yes     unknown defer' \
        'active   no  unknown unknown defer' \
        'failed   yes yes     unknown restart' \
        'failed   yes no      unknown restart' \
        'failed   no  yes     unknown defer' \
        'failed   no  no      unknown defer' \
        'inactive yes yes     unknown current' \
        'activating no yes    unknown current' \
        'unknown  no  yes     yes     current'
    do
        # shellcheck disable=SC2086
        set -- $_case
        _cases=$((_cases + 1))
        _got=$(decide "$1" "$2" "$3" "$4")
        if [ "$_got" = "$5" ]; then
            printf '  ok    %-10s %-3s %-7s %-7s -> %s\n' "$1" "$2" "$3" "$4" "$_got"
        else
            printf '  FAIL  %-10s %-3s %-7s %-7s -> %s (want %s)\n' \
                "$1" "$2" "$3" "$4" "$_got" "$5"
            _fail=1
        fi
    done
    # The table must not silently shrink to nothing — the same two-direction
    # discipline `make line-cap` and `leak-scan --self-test` hold.
    [ "$_cases" -gt 0 ] || die 'the decision table ran 0 cases: the harness is broken'
    [ "$_fail" = 0 ] || die 'the restart decision is wrong'
    say "self-test passed ($_cases cases)"
}

[ "${1:-}" != --self-test ] || { self_test; exit 0; }

# ---------------------------------------------------------------- 1. install
#
# The newest version the registry will serve. **Yanked releases are filtered
# HERE rather than left to cargo, and that is what makes a yank the rollback
# lever**: yank a bad release and the next tick resolves the previous one, sees
# it differ from what is installed, and puts it back — with nobody logging in.
# `cargo install` alone cannot do this, because it refuses to go backwards; that
# is why the install below passes an explicit `--version` and `--force`.
#
# **The fetch is a separate statement from the parse, deliberately.** As one
# pipeline the exit status would be the parser's, so a registry that answered
# 503 would parse to an empty string and look like an empty index — two
# different failures reported as one, and the wrong one.
latest_live() {
    _index=$(curl -fsS --max-time 60 "$INDEX_URL") \
        || die 'cannot reach the registry index'
    printf '%s\n' "$_index" | while IFS= read -r _line; do
        # One JSON object per line. Read it with parameter expansion rather than
        # a regex: `"vers"` occurs exactly once per line (a dependency entry
        # carries `"req"`, never `"vers"`), so there is nothing for a greedy
        # match to run past.
        case $_line in
            *'"yanked":false'*) ;;
            *) continue ;;
        esac
        _v=${_line#*'"vers":"'}
        _v=${_v%%'"'*}
        [ -n "$_v" ] && printf '%s\n' "$_v"
    done | sort -V | tail -1
}

# What version a file is, asked of the file: cargo's bookkeeping can disagree
# with what is on disk after a hand install, and the file is what the unit
# execs. Absent reads empty, which is a lawful answer — a box seated for the
# first time has no binary, and a stopped unit has no running one.
version_of() { [ -x "$1" ] || return 0; "$1" --version 2>/dev/null | awk 'NR==1 {print $NF}'; }

want=$(latest_live)
[ -n "$want" ] || die 'the registry index named no live version'
have=$(version_of "$BIN")

changed=no
if [ "$want" != "$have" ]; then
    changed=yes
    say "installing $want (was ${have:-absent})"
    # `--locked` builds against the lockfile the crate publishes, which is the
    # parity check that the foot resolves as released. `--version` is explicit
    # and `--force` is set because the move may be a DOWNGRADE — the yank lever
    # above — and cargo refuses to move backwards otherwise.
    cargo install "$CRATE" --root "$ROOT" --locked --version "$want" --force
    say "installed $(version_of "$BIN")"
else
    say "installed $have is current"
fi

# ---------------------------------------------------------------- 2. restart

state=$(systemctl --user show -P ActiveState "$UNIT" 2>/dev/null || echo unknown)
[ -n "$state" ] || state=unknown
pid=$(systemctl --user show -P MainPID "$UNIT" 2>/dev/null || echo 0)

# "A restart is pending" is not a flag anybody writes: it is the running foot
# being an older VERSION than the one installed. Each half is self-reported —
# the running foot through `/proc/<pid>/exe`, which is still the replaced file
# after an install renamed a new one over the path, and the installed binary
# through the path itself — and an unreadable half is `unknown`, which defers.
#
# **The version and not the inode, and that is the whole of bl-ad9c.** An inode
# says "some other file is there now", which is the same question only while
# this reconciler is the only writer of the install path. A hand `make install`
# is a second writer — a new inode at no new published version — and under the
# inode read the next tick restarted the foot onto it, killing whatever
# invocation was executing at the moment the box looked idle enough to act. yog
# met exactly that on a live engine (its bl-6b27). This file's rule is *the foot
# should be running the newest live version*, and after step 1 the installed
# binary IS that version, so comparing versions asks the rule itself.
pending=unknown
running_version=$(version_of "/proc/${pid:-0}/exe")
installed_version=$(version_of "$BIN")
if [ -n "$running_version" ] && [ -n "$installed_version" ]; then
    if [ "$running_version" = "$installed_version" ]; then pending=no; else pending=yes; fi
fi

# The busy read, and only when there is something to defer FOR: a foot that is
# already the installed binary will not be restarted whatever it is doing, and
# reading anyway is work spent on every tick forever.
idle=unknown
if [ "$state" = active ] && [ "$pending" = yes ]; then
    cgroup=$(systemctl --user show -P ControlGroup "$UNIT" 2>/dev/null || true)
    procs=
    [ -z "$cgroup" ] || [ ! -r "/sys/fs/cgroup${cgroup}/cgroup.procs" ] \
        || procs=$(wc -l < "/sys/fs/cgroup${cgroup}/cgroup.procs")
    case ${procs:-} in
        '')  idle=unknown ;;
        1)   idle=yes ;;
        *)   idle=no ;;
    esac
fi

case "$(decide "$state" "$changed" "$pending" "$idle")" in
    current)
        if [ "$state" = active ]; then
            say 'the running foot is the installed binary; nothing to do'
        else
            say "$UNIT is $state; leaving it alone"
        fi ;;
    defer)
        if [ "$state" = failed ]; then
            say "$UNIT is failed and $want is all the registry offers;" \
                "leaving it (journalctl --user -u $UNIT)"
        else
            say "an invocation is executing or a fact is unreadable (pending:" \
                "$pending, idle: $idle); deferring the restart to the next tick"
        fi ;;
    restart)
        say "starting $UNIT on $(version_of "$BIN") (was $state)"
        # `reset-failed` FIRST, always. A unit that tripped its start limit is
        # REFUSED a restart until the limit's interval expires — so without this
        # the recovery arm above cannot actually recover anything. On a healthy
        # unit it is a no-op beyond clearing the restart counter, which is the
        # right thing to do when putting a new version on anyway.
        systemctl --user reset-failed "$UNIT" 2>/dev/null || true
        systemctl --user restart "$UNIT" ;;
esac
