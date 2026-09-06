#!/bin/sh
# Seat the foot deployment on THIS box (bl-6c98): `make deploy-local`.
#
# It takes no argument and reaches no network but the registry. Every fact about
# the box is the box's own — the tool document and the channel directories are
# already there, put by the operator's hand and never by thrall — so pointing
# this at a second box is a different checkout, not an edit, and a box that
# should stop tracking releases is one `systemctl --user disable` away with no
# file in this tree to change. That is the severability test and the leak gate's
# rule at once: no address, account or machine name is committed anywhere here.
#
# **It seats a timer; it does not deploy a build.** Nothing is compiled here.
# The box installs from crates.io on its own schedule from then on: a foot's
# unit of install is a published version, and the registry already serves it.
#
# **It refuses a box that is not provisioned**, because a foot that cannot be a
# foot is a unit that will crash-loop into `failed` and a seating that reported
# success. `tools.json` and at least one channel are the two files thrall
# reads, both operator-authored, and their absence is the one failure this
# script can name better than the journal can.
#
# Idempotent, and the upgrade path: re-run it to move this box to this
# checkout's units and reconciler.
#
# **Its last act runs the reconciler once, synchronously, and then proves the
# foot is up** — so seating a box either ends with the newest release serving,
# or says why, rather than reporting that a timer was enabled and leaving the
# first real answer an hour away.
set -eu

self=${0##*/}
here=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
data=${XDG_DATA_HOME:-$HOME/.local/share}/thrall

say() { printf '\033[1m==>\033[0m %s\n' "$*"; }
die() { printf '%s: %s\n' "$self" "$*" >&2; exit 1; }

command -v systemctl >/dev/null 2>&1 || die 'no systemctl: this recipe seats systemd user units'
command -v cargo >/dev/null 2>&1 || die 'no cargo on PATH: the reconciler installs from crates.io'
command -v curl >/dev/null 2>&1 || die 'no curl on PATH: the reconciler reads the registry index'

[ -f "$data/tools.json" ] \
    || die "no tool document at \$XDG_DATA_HOME/thrall/tools.json: this box offers
  nothing, and a foot with no tool document refuses and says so. Write it
  first — it is the operator's, and thrall never writes it."
[ -d "$data/wire/workspaces" ] && [ -n "$(ls -A "$data/wire/workspaces" 2>/dev/null)" ] \
    || die "no channel under \$XDG_DATA_HOME/thrall/wire/workspaces/: this box is
  enrolled with no engine, and a foot with no channel has nothing to dial.
  Enrol it first — the certificate is the operator's to issue."

say 'installing the units and the reconciler'
mkdir -p "$HOME/.local/bin" "$HOME/.config/systemd/user"
# To a temp name and then `mv` into place: the reconciler may be running right
# now (the timer is armed from a previous seating), and a copy truncates before
# it writes. rename(2) in the same directory means a running shell reads
# whole-old or whole-new and never a half file.
install -m 0755 "$here/reconcile.sh" "$HOME/.local/bin/.thrall-reconcile.tmp"
mv -f "$HOME/.local/bin/.thrall-reconcile.tmp" "$HOME/.local/bin/thrall-reconcile"
install -m 0644 "$here/thrall.service" "$here/thrall-reconcile.service" \
    "$here/thrall-reconcile.timer" "$HOME/.config/systemd/user/"

# **Lingering, or the foot stops at logout** — and a box that reboots without it
# comes back with no foot at all, which the engine sees only as tools that are
# no longer offered. `enable-linger` is the one act here that is not a file
# copy, and it is what makes the unit a deployment rather than a convenience.
say 'enabling lingering so the foot outlives a logout'
loginctl enable-linger "$(id -un)" >/dev/null 2>&1 \
    || say 'could not enable lingering (the foot will stop at logout)'

say 'arming the units'
# `reset-failed` before the enable, for the reason `reconcile.sh` states: a unit
# that hit its start limit refuses to start until the interval expires, and a
# start without this is a no-op that reads as a success.
systemctl --user daemon-reload
systemctl --user reset-failed thrall.service thrall-reconcile.service 2>/dev/null || true
systemctl --user enable thrall.service thrall-reconcile.timer
systemctl --user start thrall-reconcile.timer

# The verification, and it is the reconciler itself rather than a probe of one.
# `systemctl --user start` blocks on a `Type=oneshot` unit and exits non-zero
# when it fails, so this is a real end-to-end run — the index reached, the
# version compared, the build done if there was one — and not a status print. A
# first-ever seating builds the foot here, which is minutes.
say 'running the first reconcile (a first build is not quick)'
systemctl --user start thrall-reconcile.service \
    || { journalctl --user -u thrall-reconcile.service --no-pager --lines=30 2>&1 \
             | sed 's/^/  | /' >&2
         die 'the first reconcile failed (the timer is armed; it will retry)'; }
journalctl --user -u thrall-reconcile.service --no-pager --lines=5 -o cat 2>/dev/null || true

say 'starting the foot'
systemctl --user restart thrall.service

# One bounded sleep, no polling loop. Longer than the unit's `RestartSec=5s`, so
# a foot that cannot start has already been restarted at least once by here and
# this reads a settled box rather than the instant `restart` returned.
sleep 8
state=$(systemctl --user is-active thrall.service 2>/dev/null || true)
[ "$state" = active ] || {
    journalctl --user -u thrall.service --no-pager --lines=20 2>&1 | sed 's/^/  | /' >&2
    die "thrall.service is '$state', not active"
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
# design (DESIGN §3.6) and, until bl-5d62, is redialled forever anyway. No wait
# resolves it: one of the two components has to be released. So a box in that
# state is a box with a seated timer and no foot, and saying "seated and
# serving" over it would be exactly the lie this beat exists to refuse. It is
# also self-healing without a human once both sides publish, which is the whole
# point of the timer that was just armed — so the units stay armed and only the
# report fails.
invocation=$(systemctl --user show -P InvocationID thrall.service 2>/dev/null || true)
said=$(journalctl --user _SYSTEMD_INVOCATION_ID="${invocation:-none}" \
           --no-pager --lines=20 -o cat 2>/dev/null || true)
say 'what the foot said on the way up:'
printf '%s\n' "${said:-  (nothing yet)}" | sed 's/^/  | /'

case $said in
    *'protocol mismatch'*)
        die "the foot is running and can serve nothing: it and its engine speak
  different wire versions, which is fail-closed by design and never resolves by
  waiting. The units are seated and the timer is armed, so this box adopts the
  fix by itself as soon as the older component publishes a release — there is
  nothing to do here but wait for that." ;;
esac

say "seated: $("$HOME/.local/bin/thrall" --version) is serving, and this box tracks released versions hourly"
