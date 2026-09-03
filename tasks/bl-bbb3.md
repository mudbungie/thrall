+++
title = "arm the release pipeline: a CI workflow to gate it, a trusted publisher, and the push trigger"
created = 1788146102
updated = 1788399019
claimant = "Thrallship"
priority = 2
root_commit = "32be9f81c8ae1d50610ed025db0de83568b6736b"
+++
`.github/workflows/release-plz.yml` is on disk, disarmed at the trigger
(`workflow_dispatch` only) with a first job that refuses outright, and every
other job `needs:` it. That refusal job stands **where the build gate goes**, so
the edit that arms the pipeline is the edit that gives it one — which is why the
arming was left out of the first publish (bl-d2be) rather than folded into it.

The precondition that blocked this is now gone: trusted publishing cannot CREATE
a crate, only publish a further version of one that exists. 0.0.1 exists, so the
publisher is registrable.

Its own header states the five steps and three are left. In order, each
severable:

- **a CI workflow, and `needs:` it.** This repository has no CI at all — the
  gate has only ever run on the author's box. The release job's `needs: refuse`
  becomes `needs: ci`, and the CI workflow must be a **CALLED** workflow inside
  the same run (`uses: ./.github/workflows/ci.yml`), never a `workflow_run`
  trigger: the registry refuses OIDC tokens minted under one, so a
  `workflow_run` gate would publish nothing, quietly, forever.
- **register the trusted publisher** on the registry side — one named workflow
  FILENAME in one named repository may publish this one crate — and turn on
  "Allow GitHub Actions to create and approve pull requests", without which the
  release-PR job 403s. The filename is matched literally against the OIDC claim,
  so renaming that file breaks publishing until the registry entry is updated.
  Registering it is also what retires the token path 0.0.1 used: no long-lived
  registry credential is stored in this repository and none should be.
- **add the automatic trigger**, `push: branches: [main]`.

Two artifact slots in that workflow stay deliberately empty and are not this
ball's either: no upload job for the release binary or the aarch64-apple-darwin
one, and no image push at tag time.

---

THE OPERATOR ACTS THAT REMAIN, and no edit in this tree can perform either.
Everything repo-side is done: ci.yml exists and is CALLED by release-plz.yml,
the release job is needs: ci, the push trigger is on, release-plz.toml holds
the release policy, and the actions are pinned to full commit SHAs.

1. REGISTER THE TRUSTED PUBLISHER on crates.io, on the thrall crate:
     owner       mudbungie
     repository  thrall
     workflow    release-plz.yml     (matched LITERALLY against the OIDC claim
                                      -- renaming that file breaks publishing
                                      until this entry is updated)
     environment (empty)
   This is what retires the token path 0.0.1 and 0.0.2 used. Do NOT add a
   CARGO_REGISTRY_TOKEN secret as an alternative: the workflow performs the
   OIDC exchange itself, and a long-lived registry credential in this
   repository is the thing trusted publishing exists to remove.

2. Settings -> Actions -> General:
   - tick "Allow GitHub Actions to create and approve pull requests", without
     which the release-PR job 403s;
   - set Workflow permissions to "Read and write permissions". A sibling
     repository was seen publishing to the registry and THEN 403ing on the tag
     under the fresh-repo read-only default, despite job-level contents:
     write. cargo publish is irreversible, so verify this BEFORE merging a
     release PR, not after.

Neither is silent when missing: the pipeline fails loudly, which is the
direction that was chosen.

AUTO-MERGE OF THE RELEASE PR IS NOT IMPLEMENTED and was not invented here.
Neither sibling repository carries any mechanism for it -- no auto-merge, no
automerge, no gh pr merge anywhere in their workflows or scripts -- so
adopting one is a suite-wide decision, and doing it first in the third
repository to be armed would make thrall the odd one out rather than the
pattern. Merging the release PR is the human control point in all three today.

NOT THIS BALL, deliberately: no artifact-upload job for the Linux binary or
the aarch64-apple-darwin build, and no image push at tag time -- the two slots
the workflow header names. The published-ref scan is bl-e95a, unchanged except
that the CI it needed now exists. And bl-2e63 was filed from this ball's own
gate runs: one test bounds a capture by two seconds of wall clock, which fails
on a loaded box and now sits on the release path.
