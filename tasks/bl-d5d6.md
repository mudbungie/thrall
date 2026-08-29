+++
title = "deferred: the MCP bridge, thrall as a local MCP client"
created = 1787977363
updated = 1787977363
priority = 0
root_commit = "32be9f81c8ae1d50610ed025db0de83568b6736b"

[[blockers]]
id = "bl-4cda"
on = "claim"
+++
**Deferred, not v1.** Filed so the direction is recorded and so nobody builds toward it by accident.

The shape: thrall runs as an MCP client against MCP servers on its own box, and **re-advertises their tools up the wire** as ordinary entries in its own advertisement. An MCP server's tool becomes a `{name, description, input_schema}` triple like every other, and an invocation routed to it is dispatched over MCP instead of to an argv.

Two properties are the whole point:

- **The engine never learns MCP.** MCP terminates at the foot. Upstream sees one vocabulary — advertise, invocations, complete — and gains no protocol, no verb and no transport.
- **The local config stays the gate.** An MCP server is enabled the same way a command is: by appearing in the operator-authored document on this box. Nothing is discovered and auto-advertised.

Open before this can be claimed:

- name collisions between an MCP server's tools and the argv tools in the same config — the config's own loud decline should cover it, but the composed name needs a rule
- whether an MCP server's lifetime is per invocation or per thrall process, against the serial-execution rule
- what an MCP tool's non-text content parts become, given a capture is text
- whether a generic passthrough is even representable without collapsing the grant into one bit — the reason the far end has one `clients` tool with named ops rather than a generic `call {client, tool, arguments}`