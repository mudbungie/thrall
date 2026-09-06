+++
title = "the reconciler restarts the foot on a replaced inode, so a hand install kills an invocation the next tick"
created = 1788673919
updated = 1788673922
priority = 3
root_commit = "32be9f81c8ae1d50610ed025db0de83568b6736b"
tags = ["usability-r1"]
+++
bl-6c98 reads "a restart is pending" as the inode behind the running process
exe link against the inode of the installed file. That is exactly right while
the reconciler is the only writer of the install path, and wrong the moment it
is not: a `make install` from a checkout, or any other hand install, changes the
inode without changing what version is published, and the next tick restarts the
foot onto it — killing whatever invocation is executing at the moment it finds
the box idle enough to act.

yog fixed the same defect in its native reconciler (yog bl-6b27) after a live
sighting: that repository ships `scripts/install-main`, which writes the same
path whenever the main ref moves, so a ball closing became an unattended deploy
of unreleased code onto the operator live engine. thrall has no equivalent
second writer today, which is why this is p3 and not p2 — but the proxy is wrong
for the same reason and the fix is the same three lines.

The rule this file states is *the foot should be running the newest live
version*. Read that rule directly: compare the RUNNING foot version (through its
own exe link) against the INSTALLED binary version. After the install step the
installed binary IS the newest live version, so the two comparisons agree
wherever they should and differ only on a same-version swap, which is a dev
build and not a publication.

The decision table and every arm are unchanged — only the derivation of the
`pending` fact moves — and the self-test end-to-end cases drive a zero main pid,
which stays `unknown`, so they need no edit.