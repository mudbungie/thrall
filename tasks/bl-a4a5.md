+++
title = "the mTLS channel and the foot-grade certificate"
created = 1787977353
updated = 1787977353
priority = 2
root_commit = "32be9f81c8ae1d50610ed025db0de83568b6736b"

[[blockers]]
id = "bl-e5ba"
on = "claim"
+++
The transport half. thrall **dials**; it never listens. Nothing in the path is the engine speaking first (yog REMOTE §3, §5).

- **Dial-in only.** The foot opens the connection, presents its leaf, and holds it only while waiting on its mailbox. It is absent for the whole time it is executing something, which is why presence is not the routing predicate at the far end.
- **Foot-grade leaf.** A thrall's certificate may advertise and execute only — no ask, no act. This narrows REMOTE §2's "every certificate is operator-grade within its registrations" for the class, and closes §9.6's residual (a registered client reads the trail of every workspace) for feet, without reopening §11's rejection of per-tool/per-verb ACLs: the grade is binary, not a policy layer. Enforcement is the server's; thrall's obligation is to carry a leaf of that grade and to refuse to be configured with anything else.
- **Certificates arrive out of channel.** REMOTE §1.4, verbatim and forever: the pair is carried to this box by hand. thrall mints nothing and bootstraps nothing in band.
- **The entry is the client's half** (REMOTE §2): a directory under this box carrying the channel facts that reach one workspace — the host engine's anchors, this box's leaf and key for it, the host's address, and the name the workspace bears here. Possession, where registration is the server's permission.
- **Key material lives beside any generated state, never inside it.** A reseed must not be a revocation.
- **No reconnect.** A channel that fails is an exit naming the failure. Restart policy belongs to the supervision the operator's machine already has; inventing one here would be thrall deciding how a box it does not administer runs a program.
- **Version skew is now possible for the first time**, so the handshake carries a protocol version and a mismatch refuses fail-closed, naming both versions.