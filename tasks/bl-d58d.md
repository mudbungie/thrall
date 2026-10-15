+++
title = "cargo deny fails every gate on RUSTSEC-2026-0349: rustls 0.23.45 in the lockfile; the fix is 0.23.46, a lockfile move of an approved dependency (cargo update -p rustls)"
created = 1792034238
updated = 1792034332
claimant = "Surveying"
priority = 1
root_commit = "32be9f81c8ae1d50610ed025db0de83568b6736b"
+++
Same defect lernie bl-b10b fixed: the builder's make check fails at the deny step for every tree until the lockfile carries rustls ≥ 0.23.46. No manifest change. Landing it on main here will cut a release by release-plz, which is why it is filed rather than done in passing.