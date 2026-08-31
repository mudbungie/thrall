+++
title = """two refusals are a library's diagnostics rather than the operator's: the lost-channel sentence carries a docs URL and no address, and a non-certificate leaf earns "no items found""""
created = 1788150531
updated = 1788150531
priority = 2
root_commit = "32be9f81c8ae1d50610ed025db0de83568b6736b"
+++
## What happens

Every other refusal this crate makes is written for the operator reading it.
Two are not — they are a library's internal diagnostics, surfaced verbatim.

### 1. A channel that dies mid-conversation

    thrall: <entry>: receive: peer closed connection without sending TLS
    close_notify: https://docs.rs/rustls/latest/rustls/manual/_03_howto/index.html#unexpected-eof

It names the entry and the leg, which is the useful half. It does not name the
address that went away, it puts a crate documentation URL into a supervisor's
log, and "close_notify" is a fact about TLS rather than about what an operator
should do.

Compare the sentences on the same file's other failures, which are exemplary:

    thrall: <entry>: connect <host:port>: Connection refused (os error 111)
    thrall: <entry>: connect <name:port>: failed to lookup address information: Name or service not known
    thrall: <entry>: connect <bad>: invalid socket address

Each names the address. The one for the engine going away — which is the
failure a running foot will actually hit — is the one that does not.

DESIGN §2 makes this the sentence's job:

> A channel that fails is an exit naming the failure. Restart policy belongs to
> the supervision the operator's machine already has

and `run.rs` again: *"It never reconnects. A channel that fails is the sentence
that failed it, handed back."* The sentence IS the product on this path; it is
the only thing the operator gets.

### 2. A `client.pem` that is not a certificate

    thrall: <entry>: <path>/client.pem: no items found

It locates the file, and stops there. It does not say what was expected or what
to do — next to its own sibling one line away, which does both:

    thrall: <entry>: <path>/client.pem: the leaf "<name>" is not foot grade — a
    thrall carries a certificate whose subject says OU=foot and nothing else
    (REMOTE §4.2). Mint one on the box that holds the CA.

DESIGN §3.2 says `channel::leaf::foot` "refuses bytes that are not a certificate
at all" — it does, and the refusal is the one place in the file that does not
say what the bytes should have been. `material::REMEDY` is the sentence this
wants and it is already a constant in the crate.

## Why it is worth a ball

Both are on the paths where the operator has no other information. The first is
what a supervisor's log carries when a box loses its engine; the second is what
a mis-copied file looks like. Everything else in this crate is written to be
acted on, which is what makes these two conspicuous rather than merely terse.

## Repro

1. Provision an entry, let it connect, break the connection under it.
2. Replace an entry's `client.pem` with any non-PEM bytes and run.