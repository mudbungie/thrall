+++
title = "The hourly reconcile timer never fires after a login that comes long after boot"
created = 1795582751
updated = 1795582751
root_commit = "32be9f81c8ae1d50610ed025db0de83568b6736b"
tags = ["bug"]
+++
The reconcile timer is armed by OnBootSec plus OnUnitActiveSec. OnBootSec counts from machine boot, so in a user manager that starts later than that interval after boot (a login hours after power-on) it has already elapsed and never fires; OnUnitActiveSec then has no first activation to count from. The timer sits active with no next trigger and the box silently stops taking releases, while the service keeps running a binary that has since been replaced on disk. Starting the service once by hand re-arms it.

Fix: arm from the manager's own start (OnStartupSec) or from the calendar, so a late login still gets a first run. The deploy self-test should assert the unit text.

Tested by: the timer unit carries a trigger that does not depend on boot time; the self-test fails on the old text.