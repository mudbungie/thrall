+++
title = "Collapse the pre-commit gate to exec bl-gate (rollout phase 2; ops bl-3166)"
created = 1790736019
updated = 1790736019
priority = 2
root_commit = "32be9f81c8ae1d50610ed025db0de83568b6736b"
+++
Phase 1 (bl-e66f) left an inline copy of the gate body in scripts/pre-commit. The body now lives once in ~/userconf/bin/bl-gate (~/ops/remote-builds.md 'Phase 2', 'Shared hook'): .githooks/pre-commit keeps the mainline refusal and then exec's bl-gate; scripts/pre-commit is deleted; AGENTS.md 'The gate' names bl-gate. No Makefile change: bl-gate does exactly what the inline body did (make leak-scan iff the target exists, export BALLS_TOOLCHAIN from rustc -V, bl-speculate check, bl-remote-gate).