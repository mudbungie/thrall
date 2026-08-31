+++
title = "arm the release pipeline: a CI workflow to gate it, a trusted publisher, and the push trigger"
created = 1788146102
updated = 1788146102
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