+++
title = "the first publish: the flag flips and 0.0.1 ships to the registry by hand"
created = 1788146073
updated = 1788146074
claimant = "OrderTollman"
priority = 1
root_commit = "32be9f81c8ae1d50610ed025db0de83568b6736b"
+++
The publish half of bl-006e, executed. The adjudication is made: thrall ships,
and 0.0.1 is published **by hand, with a registry token**, from `main`.

`.github/workflows/release-plz.yml` STAYS DISARMED. That is deliberate and not
an oversight: its own header names five arming steps, and steps 2, 4 and 5 (a
CI workflow the release job `needs:` as a CALLED workflow, the trusted
publisher, the push trigger) are a separate ball. Trusted publishing cannot
CREATE a crate that does not already exist, so a first version has to be
published by hand regardless — this ball is exactly that act, and it is what
makes the trusted publisher registrable at all.

What lands here:

- `publish = false` -> `true`, and the manifest comment rewritten from "the
  default is the one that cannot cost anything" to what is now true: the
  decision is made, the allowlist and the guard stand, and the remaining
  irreversibility is per-VERSION rather than per-crate.
- `repository` added. `cargo package` warns that the manifest names no
  documentation, homepage or repository, and a version's metadata is frozen at
  publication — 0.0.1 is the only chance to give the first published thrall a
  link back to its source. The repository is public as of bl-006e's remote
  half, so there is nothing to disclose by naming it.
- the manifest comment's pointer to the guard corrected: it named
  `src/packaged.rs`, and the file is `src/packaged_tests.rs`.

## The publication checklist, per item

Run in full before the flip. Each verdict is evidence, not a checkbox.

1. **History.** Every reachable commit of the trunk swept with the repo's own
   scanner in `--commit` scope (blobs added or rewritten, plus the message):
   clean, no findings. Swept again over the published task-store branch: clean.
   The scanner's own both-direction self-test passes first, so a silent scan is
   not being read as a clean one.
2. **Other refs.** Three refs exist on the remote and each has an owner: the
   trunk, the task store, and the remote's default-branch pointer. The balls
   LANDING branch is local-only and is not published — which matters, because
   it is the one ref that carries a finding: its `conf set task-remote` commit
   message quotes an SSH remote URL, which the scanner reads as an address, and
   its op trailer carries a local account name. Publishing that branch would be
   a disclosure event; nothing does, and nothing should.
3. **Commit messages.** The `commit-msg` hook runs the same scanner over the
   message, and item 1's sweep re-read every message in history independently.
4. **Repository text nobody committed.** No pull requests, no issues, no
   releases, no discussions. The repository description is one line and states
   the role. The registry description is the manifest's and states the same.
5. **Actions logs and artifacts.** No workflow has ever run — the only workflow
   on disk has no automatic trigger and a first job that refuses. Nothing to
   survive a run.
6. **Already-published versions.** The registry holds exactly one version, the
   0.0.0 placeholder, unyanked, 770 bytes, no repository link. It is not
   touched. `cargo package --list` was audited by hand against the allowlist
   before the flip: 55 entries, every one of them either a cargo-minted file,
   one of the two files a registry renders, or a `.rs` file under `src`. Zero
   strays — in particular the fabricated-secret fixture corpus, which an
   unanchored `README.md` pattern would have shipped, is absent.

The guard (`src/packaged_tests.rs`) asks the same question mechanically on
every gate run; item 6 is the human half it cannot do, because a guard judges
file classes and never content.