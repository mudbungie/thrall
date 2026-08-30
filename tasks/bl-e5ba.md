+++
title = "dependency approval: what the real client links"
created = 1787977350
updated = 1788059990
claimant = "OrderSmith"
priority = 3
root_commit = "32be9f81c8ae1d50610ed025db0de83568b6736b"

[[blockers]]
id = "bl-349f"
on = "claim"
+++
The skeleton has **zero** dependencies, deliberately. The client cannot. This ball is the operator adjudication of the list before any of it enters `Cargo.toml`, per the house rule "zero new dependencies without explicit user approval".

Proposed, each with why:

- **`rustls`** (`default-features = false`, features `ring`, `std`, `tls12`, `logging`) — the wire is mTLS and nothing else may provide it. `default-features = false` is load-bearing: rustls' defaults select `aws-lc-rs`, whose `aws-lc-sys` builds C and breaks the single-binary story. `deny.toml` bans `openssl-sys`, `native-tls` and `aws-lc-sys` so a manifest edit that drops the flag fails the gate instead of quietly re-acquiring a C toolchain.
- **`serde_json`** — the wire is JSON and a tool's `input_schema` is JSON Schema carried verbatim. Hand-rolling a parser for a security boundary is worse than a dependency.
- **`clap`** — only if the verb surface outgrows a hand-rolled argv match. The skeleton's does not; the client's may not either. Defer until a verb needs it.
- **`thiserror`** — error enums, house standard rule 10. `anyhow` is not proposed.

Explicitly NOT proposed: `tokio` (the wire is synchronous; house rule 8 stays vacuous — do not add tokio to satisfy a rule that matches nothing), `rcgen` (certificates are minted out of channel by the operator, never by a library thrall links).

Deliverable: an approved list, then a `Cargo.toml` and a `deny.toml` allow-list exhaustive over the resulting lockfile.