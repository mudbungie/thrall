+++
title = "docs/tools.example.json is outside the crate include allowlist, so bl-102b's example does not reach anyone who installs from crates.io"
created = 1788746115
updated = 1788746289
claimant = "Cantaloups-T5"
priority = 3
root_commit = "32be9f81c8ae1d50610ed025db0de83568b6736b"
tags = ["usability-r2"]
+++
Round-2 devadmin lane, re-driving bl-102b ("tools.json has no example anywhere").

## What landed, and it is good

`docs/tools.example.json` now exists and is a complete two-entry document, one
of them showing `subject_cwd`. `README.md`'s "The tool document" states the
contract in the one sentence it needed:

    **The contract is stdin, stdout, exit code**, and it is the thing to know
    before writing a single entry: the invocation's `input` JSON — the object
    the model produced, whole — arrives on the command's **standard input**;
    whatever the command writes to **standard output** is the capture the model
    reads; its **exit code** is the verdict, `0` for a tool that worked.

I authored two tool documents for two different boxes from that paragraph alone,
with no source reading, and both were accepted first try. bl-102b is
substantially fixed.

## The residual

The example file does not ship. `Cargo.toml`:

    include = [
        "/src/**/*.rs",
        "/README.md",
        "/LICENSE",
    ]

So `cargo install thrall` — the install route the README itself teaches — puts
the binary on the box and leaves `docs/tools.example.json` in the repository.
A user who installs from crates.io and never clones has the README's prose and
the README's inline document, which is most of the value, but not the file the
ball delivered, and not the `subject_cwd` entry that only the example carries.

The allowlist is correct in shape and the manifest's reasoning for an allowlist
over an exclude is right; this is one missing entry, not a policy question.

## Ask

`"/docs/tools.example.json"` in the include list — or the whole of `/docs/**`
if the design doc is wanted with it. `src/packaged_tests.rs` reads the real
`cargo package --list`, so the addition is testable in the direction that
matters.

p3: the README carries the substance, so nothing is unusable; it is the one
artifact of the fix that the fix's own audience cannot see.