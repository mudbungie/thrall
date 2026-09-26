+++
title = "the foot half of the punched wire: rendezvous material on an entry, the four-rung dial ladder, held punched connections with ping discard and the silence hangup, and a redial that re-enters the ladder"
created = 1790392652
updated = 1790392653
claimant = "Urinalyses-R"
priority = 2
root_commit = "32be9f81c8ae1d50610ed025db0de83568b6736b"
+++
The foot's half of yog bl-0247 (REMOTE §13.2–§13.4, §13.7; the engine end landed as yog bl-4263). The app's half is Kotlin in its own repository.

## What lands

1. **Entry material grows two files beside `ca.pem`** — `rendezvous.pub` (the engine's ed25519 public key, 32 bytes hex) and `pairing.salt` (32 bytes hex). Both or neither: half is the same refusal a half-provisioned entry earns. Neither = today's entry, no roving, not a failure. The foot mints nothing and holds no keypair of its own: the inbox is signed under a keypair HKDF-derived from the pairing salt, exactly as the engine derives it (yog `src/wire/rendezvous/material.rs`).

2. **The four-rung ladder** in `Channel::ask`: the live held connection → the entry's direct `address` under a bounded connect → a re-punch at the RAM-cached engine endpoints (no DHT round trip) → the full rendezvous (presence get, sealed call put, simultaneous open). Whichever rung answered, the inner mTLS verifies the same engine name the direct dial verifies, off `address`, which stays the name's one home.

3. **Redial becomes load-bearing** (thrall bl-916d built the loop; yog bl-0a74 filed it). A dropped channel re-enters the ladder on every dial with the existing backoff, so cellular flapping is answered by re-punch at cached endpoints and then by full rendezvous. What is HELD is what was punched: a direct connection stays one per ask (DESIGN §3.7's absent-while-executing reasoning), a punched one is held and reused across asks.

4. **Held-connection handling**: `{"ping":true}` is discarded wherever a reply stream is read; two minutes of silence on the socket is the hangup and it triggers the redial.

## Substrate, reimplemented per component (REMOTE §13.7 ruling 2)

A pure BEP 5 / BEP 44 client over `std::net` UDP — bencode, KRPC query shapes, ed25519-signed mutable items, the iterative walk, get/put — mirrored from yog's `src/dht`, loopback-fake tested, never a node. Crypto through `ring` (rustls' provider, already in the lockfile; named directly as yog bl-df31 did). `socket2 =0.6.5` for SO_REUSEADDR/SO_REUSEPORT on the punch port — the one new crate, approved by operator ruling 2026-09-23 (REMOTE §13.7 ruling 1).

## Tests

Seal/unseal, HKDF derivations, signed items and targets against exact byte fixtures produced by yog's own code, so interoperation is asserted rather than assumed; each rung taken and falling through against a fake DHT on loopback UDP and a fake punched engine; redial after a dropped held connection; ping discard; the silence hangup with the bound shortened. No network in the suite.