#!/bin/sh
# The regression half of the foot reconciler (bl-6c98) — `make deploy-selftest`.
#
# **It drives the REAL `reconcile.sh`**, end to end, under a fake `curl`, a
# fake `cargo` and a fake `systemctl` on `PATH` and a scratch `HOME`. Nothing
# here re-implements the decision: a self-test that restated the rule would
# prove only that the copy still agrees with itself, which is the failure the
# leak gate's own self-test is written to avoid. Every assertion below is about
# what the shipped file did.
#
# It touches no machine and needs no network, no registry, no toolchain and no
# release, which is what lets it run in the gate — and that is the point. A
# reconciler is unattended code on somebody's box, so the failures that matter
# are the quiet ones: it stops installing, or it starts installing the wrong
# thing, or it stops restarting, and nobody finds out for a month.
#
# BOTH DIRECTIONS, and that is the shape of the table rather than a footnote.
# Half the cases assert an install or a restart HAPPENED and with exactly which
# arguments; the other half assert `cargo` was never invoked at all, or that the
# unit was never touched. A reconciler that installs on every tick, and one that
# has quietly stopped, are both broken and only one of them is loud.
#
# The pure decision is checked by the shipped script's own `--self-test`, run
# first below — the restart arms that need a live `/proc/<pid>/exe` cannot be
# reached from a fake world, and a table over the function is how they are
# proved. What the end-to-end cases add is that the facts fed to that function
# are read correctly off a box.
set -eu

HERE=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
SCRIPT="$HERE/reconcile.sh"
[ -x "$SCRIPT" ] || { echo "deploy-selftest: $SCRIPT is not executable" >&2; exit 1; }

fails=0
cases=0

echo 'deploy-selftest: the decision table'
"$SCRIPT" --self-test | sed 's/^/  /'

# The synthetic index. Real sparse-index lines carry a full `deps` array and a
# checksum; the reconciler reads `"vers"` and `"yanked"` by parameter expansion
# and nothing else, so the fixture keeps the one field a greedy match could run
# past — a dependency's `"req"` — and omits the rest.
line() { # <version> <yanked:true|false>
    printf '{"name":"thrall","vers":"%s","deps":[{"name":"rustls","req":"^0.23"}],' "$1"
    printf '"features":{},"yanked":%s}\n' "$2"
}

current()    { line 0.0.4 false; line 0.0.5 false; }
tip_yanked() { line 0.0.4 false; line 0.0.5 true;  }
all_yanked() { line 0.0.4 true;  line 0.0.5 true;  }

# Run the reconciler in a built world and hand the result to a checker, which
# reads five globals: `$code` its exit status, `$out` its combined output, `$log`
# the fake cargo's recorded argv, `$units` the fake systemctl's recorded argv,
# and `$root` the install root that world's `$HOME` implies.
run_case() { # <label> <index-fn> <installed-version-or-empty> <unit-state> <main-pid> <checker-fn>
    _label=$1 _fixture=$2 _have=$3 _state=$4 _pid=$5 _check=$6
    cases=$((cases + 1))
    _work=$(mktemp -d "${TMPDIR:-/tmp}/thrall-selftest.XXXXXX")
    mkdir -p "$_work/bin" "$_work/home/.local/bin"
    "$_fixture" > "$_work/index.txt"
    root="$_work/home/.local"

    # `curl`: serves the fixture for the sparse-index URL and refuses anything
    # else with curl's own exit 22, so a reconciler that started fetching a
    # second thing fails here rather than passing silently.
    cat > "$_work/bin/curl" <<EOF
#!/bin/sh
for a in "\$@"; do :; done
[ "\$a" = https://index.crates.io/th/ra/thrall ] \
    || { echo "fake curl: unexpected URL \$a" >&2; exit 22; }
exec cat "$_work/index.txt"
EOF
    # `cargo`: records its whole argv, then emulates the install by writing the
    # binary the reconciler re-reads afterwards. Emulating it is what makes the
    # final report line an assertion rather than a hope.
    cat > "$_work/bin/cargo" <<EOF
#!/bin/sh
printf '%s\n' "\$*" >> "$_work/cargo.log"
_root=; _vers=
while [ \$# -gt 0 ]; do
  case \$1 in --root) _root=\$2; shift ;; --version) _vers=\$2; shift ;; esac
  shift
done
mkdir -p "\$_root/bin"
printf '#!/bin/sh\necho "thrall %s"\n' "\$_vers" > "\$_root/bin/thrall"
chmod 0755 "\$_root/bin/thrall"
EOF
    # `systemctl`: answers the two `show -P` reads off this case's fixture and
    # records every other invocation, so "the unit was restarted" and "the unit
    # was never touched" are both assertions about a recorded fact.
    cat > "$_work/bin/systemctl" <<EOF
#!/bin/sh
case "\$*" in
  *"show -P ActiveState"*) echo "$_state"; exit 0 ;;
  *"show -P MainPID"*)     echo "$_pid"; exit 0 ;;
esac
printf '%s\n' "\$*" >> "$_work/units.log"
EOF
    chmod 0755 "$_work/bin/curl" "$_work/bin/cargo" "$_work/bin/systemctl"

    if [ -n "$_have" ]; then
        printf '#!/bin/sh\necho "thrall %s"\n' "$_have" > "$root/bin/thrall"
        chmod 0755 "$root/bin/thrall"
    fi

    set +e
    out=$(HOME="$_work/home" PATH="$_work/bin:$PATH" "$SCRIPT" 2>&1)
    code=$?
    set -e
    log=$(cat "$_work/cargo.log" 2>/dev/null || true)
    units=$(cat "$_work/units.log" 2>/dev/null || true)

    if "$_check"; then
        printf '  ok    %s\n' "$_label"
    else
        printf '  FAIL  %s (exit %s)\n' "$_label" "$code"
        printf '%s\n' "$out" | sed 's/^/          | /'
        printf '        cargo: %s\n' "${log:-<never invoked>}"
        printf '        units: %s\n' "${units:-<never touched>}"
        fails=1
    fi
    rm -rf "$_work"
}

# The exact argument vector, not merely "cargo ran". `--version` with `--force`
# IS the yank lever's mechanism — without both, cargo refuses to move backwards
# and a rollback silently does nothing — and `--root` is what keeps the new
# build on the path the UNIT execs, which is the difference between a CD and a
# timer updating a binary nothing runs.
installed_vector() { # <version>
    [ "$log" = "install thrall --root $root --locked --version $1 --force" ]
}
said() { printf '%s' "$out" | grep -q "$1"; }
# `grep -q` from a herestring is not available in POSIX sh; the subject is fed
# by a pipe, so the writer here is `printf` and not a process whose death could
# be mistaken for a failure — there is no `pipefail` in this shell.
never_restarted() { [ -z "$units" ]; }
restarted() {
    printf '%s' "$units" | grep -q '^--user reset-failed thrall.service$' \
        && printf '%s' "$units" | grep -q '^--user restart thrall.service$'
}

installed_5()   { [ "$code" = 0 ] && installed_vector 0.0.5; }
bootstrapped()  { installed_5 && said 'installing 0\.0\.5 (was absent)'; }
rolled_back()   { [ "$code" = 0 ] && installed_vector 0.0.4 \
                    && said 'installing 0\.0\.4 (was 0\.0\.5)'; }
no_install()    { [ "$code" = 0 ] && [ -z "$log" ] && said 'installed 0\.0\.5 is current'; }
refused_with()  { [ "$code" != 0 ] && [ -z "$log" ] && said "$1"; }
refused_nothing_live() { refused_with 'named no live version'; }

# --- the install half, both directions ---------------------------------------
# `inactive` throughout: the restart half is exercised below, and a stopped unit
# is the arm that must leave the unit alone whatever it installed.
run_case 'behind the registry -> installs the newest live version' \
    current 0.0.4 inactive 0 installed_5
# A box with no binary at all is that same question with an empty answer rather
# than a special case: `installed` yields the empty string and the compare
# differs, so the bootstrap and the upgrade are one path.
run_case 'no binary at all -> installs, reporting "was absent"' \
    current '' inactive 0 bootstrapped
# The negative arm. Nothing to do must mean nothing done.
run_case 'already newest -> does NOT invoke cargo' \
    current 0.0.5 inactive 0 no_install
# The rollback lever, whole: a yank makes the previous version newest-live, the
# compare sees it differ from what is installed, and the install goes BACKWARDS.
run_case 'newest yanked -> rolls the box back a version' \
    tip_yanked 0.0.5 inactive 0 rolled_back
run_case 'nothing live at all -> refuses, installs nothing' \
    all_yanked 0.0.5 inactive 0 refused_nothing_live

# --- the restart half, both directions ---------------------------------------
restart_on_failed() { installed_5 && restarted; }
left_alone()        { installed_5 && never_restarted && said 'is inactive; leaving it alone'; }
deferred_unknown()  { installed_5 && never_restarted && said 'deferring the restart'; }

# A unit that could not start has no turn to protect, and the version it failed
# on is not the one being put on it. This is the arm that recovers a box from a
# release that crashes on boot, with nobody logging in.
run_case 'failed unit + a new version -> resets and restarts it' \
    current 0.0.4 failed 0 restart_on_failed
# Stopped on purpose is not ours to move, however far behind it is.
run_case 'stopped unit -> installs but never touches it' \
    current 0.0.4 inactive 0 left_alone
# An `active` unit with no readable main pid is a fact we could not read, and a
# fact we could not read is never grounds for killing a turn we cannot see.
run_case 'active but the pid is unreadable -> defers, touches nothing' \
    current 0.0.4 active 0 deferred_unknown

# The registry unreachable. A refusing `curl` and an EMPTY body are different
# failures and must not report as one, so this drives the first: the shim exits
# 22 the way curl does, and the reconciler must name the registry rather than
# the index's contents.
cases=$((cases + 1))
work=$(mktemp -d "${TMPDIR:-/tmp}/thrall-selftest.XXXXXX")
mkdir -p "$work/bin" "$work/home"
printf '#!/bin/sh\nexit 22\n' > "$work/bin/curl"
printf '#!/bin/sh\nprintf "%%s\\n" "$*" >> "%s/cargo.log"\n' "$work" > "$work/bin/cargo"
printf '#!/bin/sh\nprintf "%%s\\n" "$*" >> "%s/units.log"\n' "$work" > "$work/bin/systemctl"
chmod 0755 "$work/bin/curl" "$work/bin/cargo" "$work/bin/systemctl"
set +e
out=$(HOME="$work/home" PATH="$work/bin:$PATH" "$SCRIPT" 2>&1); code=$?
set -e
log=$(cat "$work/cargo.log" 2>/dev/null || true)
units=$(cat "$work/units.log" 2>/dev/null || true)
if refused_with 'cannot reach the registry index' && never_restarted; then
    printf '  ok    %s\n' 'registry unreachable -> refuses, changes nothing'
else
    printf '  FAIL  %s (exit %s)\n' 'registry unreachable -> refuses, changes nothing' "$code"
    printf '%s\n' "$out" | sed 's/^/          | /'
    fails=1
fi
rm -rf "$work"

# The empty-set guard, the same two-direction discipline `make line-cap` and
# `make rules-audit` hold: a table that ran no case is a broken harness, not a
# clean reconciler, and it must not pass as green.
[ "$cases" -gt 0 ] \
    || { echo 'deploy-selftest: ran 0 cases — the harness is broken' >&2; exit 1; }
[ "$fails" = 0 ] || { echo 'deploy-selftest: the reconciler is wrong' >&2; exit 1; }
echo "deploy-selftest: $cases cases, all passed"
