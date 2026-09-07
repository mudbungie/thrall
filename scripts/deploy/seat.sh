#!/bin/sh
# Seat the foot deployment (bl-6c98; the remote carrier is bl-605f):
#
#   make deploy-local            # seat THIS box
#   make deploy HOST=<ssh-host>  # seat that one
#
# **Two carriers, one payload.** The files and the arming commands are spelled
# once and do not know which way they arrived; only `put` and `run` below differ.
# That is the idiom lernie's `scripts/deploy/seat.sh` states, and it is the whole
# reason the remote form is not a second recipe: two statements of one seating
# drift, and the copy that drifts is the one nobody re-reads.
#
# **The remote carrier is not a convenience, it is the only door for the box
# class that most needs a foot.** Until bl-605f a foot could be deployed only by
# a human logged in to the machine that would run it, so the always-on server —
# the box that is awake when the laptop is not, and the only box that can act on
# ITSELF — had no foot at all, while its engine held a certificate issued for
# somebody's laptop. Neither end could say so: REMOTE §5.1 stores one advertised
# set per identity and §3.7 rules that a working foot is ABSENT from the
# engine's view, so a foot that is not running looks like a foot that has not
# spoken lately, and §3.1 forbids the engine probing.
#
# HOST, when given, is an ssh destination and the ONLY parameter — no address,
# account or machine name is committed anywhere in this tree. That is the leak
# gate's rule and the severability one at once: pointing this at a second box is
# a different argument, not an edit, and a box that should stop tracking releases
# is one `systemctl --user disable` away with no file here to change.
#
# **It seats a timer; it does not deploy a build.** Nothing is compiled here and
# nothing is carried over the channel but four small text files. The box installs
# from crates.io on its own schedule from then on: a foot's unit of install is a
# published version, and the registry already serves it. That is also why the
# preflight below insists on a toolchain ON THE TARGET — a box with no cargo can
# hold a foot binary but cannot pick up the next release, and a seating that
# armed a timer which could never install anything would be reporting CD it
# cannot perform.
#
# **It refuses a box that is not provisioned**, because a foot that cannot be a
# foot is a unit that crash-loops into `failed` behind a seating that reported
# success. `tools.json` and at least one channel are the two things thrall reads,
# both operator-authored and neither ever written by thrall, and their absence is
# the one failure this script can name better than the journal can.
#
# Idempotent, and the upgrade path: re-run it to move a box to this checkout's
# units and reconciler.
#
# **Its last act runs the reconciler once, synchronously, and then proves the
# foot is up** — so seating a box either ends with the newest release serving, or
# says why, rather than reporting that a timer was enabled and leaving the first
# real answer an hour away on a machine nobody is watching.
set -eu

self=${0##*/}
host=${1:-}
here=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
where=${host:-this box}

say() { printf '\033[1m==>\033[0m %s\n' "$*"; }
die() { printf '%s: %s\n' "$self" "$*" >&2; exit 1; }

# The carriers, and the whole of what a HOST changes. `run` takes one shell
# command and runs it ON THE TARGET; `put` takes a source file and a destination
# path RELATIVE to the home directory, so the payload never has to know whose
# home it is. Every fact the payload reads — the data root, the units, the
# journal — is therefore the target's own, not this checkout's box's.
if [ -n "$host" ]; then
    run() { ssh -n "$host" "$1"; }
    put() { scp -q "$1" "$host:$2"; }
else
    run() { sh -c "$1"; }
    put() { install -m 0644 "$1" "$HOME/$2"; }
fi

say "checking that $where can be a foot"
run 'command -v systemctl >/dev/null 2>&1' \
    || die "no systemctl on $where: this recipe seats systemd user units"
# **Asked the way the RECONCILER will ask it.** `thrall-reconcile.service` puts
# `%h/.cargo/bin` on the unit's PATH, because that is where rustup installs and a
# login shell learns it from `~/.cargo/env` — which `ssh <host> <command>` never
# reads. A preflight that trusted this session's PATH would refuse nearly every
# rustup box over the remote carrier while the unit ran on it perfectly.
run 'command -v cargo >/dev/null 2>&1 || [ -x "$HOME/.cargo/bin/cargo" ]' \
    || die "no cargo on $where: the reconciler installs the foot from crates.io,
  so a box with no toolchain can hold a binary but can never pick up a release"
run 'command -v curl >/dev/null 2>&1' \
    || die "no curl on $where: the reconciler reads the registry index with it"
run 'test -f "${XDG_DATA_HOME:-$HOME/.local/share}/thrall/tools.json"' \
    || die "no tool document at \$XDG_DATA_HOME/thrall/tools.json on $where: that
  box offers nothing, and a foot with no tool document refuses and says so.
  Write it first — it is the operator's, and thrall never writes it."
run 'ls -A "${XDG_DATA_HOME:-$HOME/.local/share}/thrall/wire/workspaces" 2>/dev/null | grep -q .' \
    || die "no channel under \$XDG_DATA_HOME/thrall/wire/workspaces/ on $where:
  that box is enrolled with no engine, and a foot with no channel has nothing to
  dial. Enrol it first — the certificate is the operator's to issue."

say "seating the units and the reconciler on $where"
run 'mkdir -p "$HOME/.local/bin" "$HOME/.config/systemd/user"'
# To a temp name and then `mv` into place: the reconciler may be running right
# now (the timer is armed from a previous seating), and both carriers truncate
# before they write. rename(2) in the same directory means a running shell reads
# whole-old or whole-new and never a half file.
put "$here/reconcile.sh" .local/bin/.thrall-reconcile.tmp
put "$here/thrall.service" .config/systemd/user/thrall.service
put "$here/thrall-reconcile.service" .config/systemd/user/thrall-reconcile.service
put "$here/thrall-reconcile.timer" .config/systemd/user/thrall-reconcile.timer
run 'chmod 0755 "$HOME/.local/bin/.thrall-reconcile.tmp" && \
    mv -f "$HOME/.local/bin/.thrall-reconcile.tmp" "$HOME/.local/bin/thrall-reconcile"'

# **Lingering, or the foot stops at logout** — and a box that reboots without it
# comes back with no foot at all, which the engine sees only as tools that are no
# longer offered. It is the one act here that is not a file copy, and it is what
# makes the unit a deployment rather than a convenience.
say "enabling lingering so the foot outlives a logout on $where"
run 'loginctl enable-linger "$(id -un)" >/dev/null 2>&1' \
    || say 'could not enable lingering (the foot will stop at logout)'

say "arming the units on $where"
# `reset-failed` before the enable, for the reason `reconcile.sh` states: a unit
# that hit its start limit refuses to start until the interval expires, and a
# start without this is a no-op that reads as a success.
run 'systemctl --user daemon-reload; \
    systemctl --user reset-failed thrall.service thrall-reconcile.service 2>/dev/null; \
    systemctl --user enable thrall.service thrall-reconcile.timer; \
    systemctl --user start thrall-reconcile.timer'

# The verification, and it is the reconciler itself rather than a probe of one.
# `systemctl --user start` blocks on a `Type=oneshot` unit and exits non-zero
# when it fails, so this is a real end-to-end run — the index reached, the
# version compared, the build done if there was one — and not a status print. A
# first-ever seating builds the foot there, which is minutes.
say "running the first reconcile on $where (a first build is not quick)"
run 'systemctl --user start thrall-reconcile.service' \
    || { run 'journalctl --user -u thrall-reconcile.service --no-pager --lines=30' 2>&1 \
             | sed 's/^/  | /' >&2
         die "the first reconcile failed on $where (the timer is armed; it will retry)"; }
run 'journalctl --user -u thrall-reconcile.service --no-pager --lines=5 -o cat' || true

say "starting the foot on $where"
run 'systemctl --user restart thrall.service'

# One bounded sleep, no polling loop. Longer than the unit's `RestartSec=5s`, so
# a foot that cannot start has already been restarted at least once by here and
# this reads a settled box rather than the instant `restart` returned.
sleep 8
state=$(run 'systemctl --user is-active thrall.service' || true)
[ "$state" = active ] || {
    run 'journalctl --user -u thrall.service --no-pager --lines=20' 2>&1 | sed 's/^/  | /' >&2
    die "thrall.service is '$state', not active on $where"
}
# **`is-active` on its own says a process exists, not that it is a foot** — the
# defect yog's own deploy was written for (its bl-0719: a deploy that prints
# success over a crash loop has reinvented the blindness it exists to remove).
# thrall writes no line when a channel OPENS, so the positive cannot be read
# here; what can be read is the one refusal that will never clear on its own.
#
# A **protocol mismatch** is that refusal. Every other failure this journal
# carries is transient by construction — a dropped channel is redialled with
# backoff, an engine that is down comes back — but a mismatch is fail-closed by
# design (DESIGN §3.6): no wait resolves it, because one of the two components
# has to publish. So a box in that state is a box with a seated timer and no
# foot, and saying "seated and serving" over it would be exactly the lie this
# beat exists to refuse. It is also self-healing without a human once both sides
# publish, which is the whole point of the timer that was just armed — so the
# units stay armed and only the report fails.
said=$(run 'invocation=$(systemctl --user show -P InvocationID thrall.service 2>/dev/null); \
    journalctl --user _SYSTEMD_INVOCATION_ID="${invocation:-none}" --no-pager --lines=20 -o cat' \
    2>/dev/null || true)
say "what the foot said on the way up:"
printf '%s\n' "${said:-  (nothing yet)}" | sed 's/^/  | /'

case $said in
    *'protocol mismatch'*)
        die "the foot on $where is running and can serve nothing: it and its
  engine speak different wire versions, which is fail-closed by design and never
  resolves by waiting. The units are seated and the timer is armed, so that box
  adopts the fix by itself as soon as the older component publishes a release —
  there is nothing to do here but wait for that." ;;
    # A dial that has not landed yet is NOT a refusal to report: an engine that
    # is down, a name that has not resolved and a listener not yet up are all
    # transient by construction, and §3.8's backoff is what they are for. But it
    # is not "serving" either, and the first live run of this script said exactly
    # that over a foot redialling a refused port — the blindness this whole
    # section exists to refuse, arriving through the one arm that had no case.
    # So it is a notice, and the last line below no longer claims service that
    # thrall writes no line to establish.
    *'Dialling this engine again'*)
        say "note: no channel is open yet — the foot is still dialling. Its own
  words are above; the units are seated and it keeps trying." ;;
esac

say "seated: $where is running $(run '"$HOME/.local/bin/thrall" --version') and tracks released versions hourly"
