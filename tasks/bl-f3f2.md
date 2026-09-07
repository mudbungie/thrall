+++
title = "every release PR leaves a permanently red CI / pull_request run: auto-merge deletes the head branch seconds after the PR opens, and release-plz.yml's header still says those runs never started a job"
created = 1788674753
updated = 1788745681
claimant = "Cantaloups-T2"
priority = 3
root_commit = "32be9f81c8ae1d50610ed025db0de83568b6736b"
tags = ["usability-r2"]
+++
Seen on PRs #5–#8 (thrall 0.0.7→0.0.10, 2026-09-06): each release PR's 'CI / pull_request' run is zero jobs with 'workflow file issue' because merge-release-pr deletes the head branch about 2 s after the PR opens. A permanent red per release can mask a real one, and the header comment in release-plz.yml is stale. Either exclude release-plz branches from ci.yml's pull_request trigger, or let the merge wait for the run, and fix the comment. Same shape likely in yog/brazen/litany/lernie — check each.