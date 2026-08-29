+++
title = "the client loop: advertise, wait on the mailbox, hand off"
created = 1787977358
updated = 1787977358
priority = 2
root_commit = "32be9f81c8ae1d50610ed025db0de83568b6736b"

[[blockers]]
id = "bl-a4a5"
on = "claim"

[[blockers]]
id = "bl-05fe"
on = "claim"
+++
The protocol half of the foot: three of the four boundary verbs a tool host touches (yog REMOTE §5.1, §5.3), and the loop that spends them.

    advertise -> { invocations -> execute -> complete } forever

- **`advertise`** carries one field, `tools`, an array whose element is exactly three facts: `name`, `description`, `input_schema`. It is the projection of the local config (its own ball) with the local half dropped.
- **`advertise` names no client, and that is the gesture.** The identity a set lands under is the intake's — the connection's certificate common name. A `client` field on the wire would let any connection overwrite any other's set, which is the authorization the certificate has already decided. The same holds for `invocations` and `complete`: a connection drains its own queue and answers its own invocations.
- **`invocations`** is the follow-class read that waits for this machine's next work. It is where the foot spends nearly all of its life, and holding it open is the only reason a foot holds a connection at all.
- **`complete`** answers exactly one invocation. A completion quoting an invocation addressed to another machine earns the sentence a handle nobody minted earns — absent, not forbidden: a refusal that confirmed existence would be a disclosure.
- **The loop is serial**, one invocation at a time. That is what makes a busy host *absent* at the far end, and it is why presence is not the routing predicate there: the mailbox queue is.
- **The advertisement is re-presented on every connect**, and the far end writes it only when it differs from what is stored, so a reconnect touches no mtime and no model's cached prefix.

Nothing here is thrall speaking unprompted. Every leg is a reply to something the foot asked for.