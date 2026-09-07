+++
title = "thrall mcp pin -- <server argv>: discover once, at operator time, and print the entries the operator vouches for; the fetch server is the shipped example"
created = 1788744423
updated = 1788744423
priority = 2
root_commit = "32be9f81c8ae1d50610ed025db0de83568b6736b"
tags = ["usability-r2"]
+++
Child 2 of the MCP bridge design (DESIGN §6, bl-3b03). One lane-day. Shares
`src/mcp/rpc.rs` with child 1; claim after it lands or coordinate on the file.

CONTRACT

`thrall mcp pin -- <server argv...>` is an OPERATOR verb, run by a human on the
box (the same posture as minting a certificate): it spawns the server once
through the spawn boundary, performs `initialize`, requests `tools/list`, and
prints to STDOUT one JSON array of complete tool-document entries — for every
tool in the catalog:

    {"name": <MCP name verbatim>, "description": <MCP description>,
     "input_schema": <inputSchema verbatim>,
     "command": [<absolute path of this binary>, "mcp", <name>, "--", <server argv...>]}

then tears the server down. It NEVER writes `tools.json`: the document is
operator-authored (DESIGN §3.4), and the allowlist IS the paste — the operator
copies in the entries they vouch for and no others (DESIGN §6.3; litany's
bridge doc §7: never pin a server's whole catalog). Any prefixing is the
operator's edit to `name`; a name that is not a single path component, or that
collides with an entry already in the document, is refused where every such
name is refused today — `config::read`, at the next `thrall run`.

To STDERR, one line per tool: the name and its MCP `annotations` when the
server sent any (`readOnlyHint`, `destructiveHint`, `idempotentHint`,
`openWorldHint`), as commentary for the operator's policy row on the engine
side (yog capability.yaml `rules:` keyed on the host-qualified name — the yog
child of this design). Annotations are hints from an untrusted server and are
NEVER placed in the entry or advertised: REMOTE §5.1 admits nothing yog stores
and cannot check.

The absolute path is `std::env::current_exe()`; a pin run from a build tree
prints that tree's path, which is correct and visible.

THE SHIPPED EXAMPLE (the web tool, bl-4409's answer under DESIGN §6.9):
`docs/tools.example.json` gains a third entry, `fetch`, pinned from the
reference `mcp-server-fetch` (Model Context Protocol reference servers; runs
under `uvx`), with its real description and `inputSchema` (`url` required;
`max_length`, `start_index`, `raw` optional) and
`"command": ["/usr/local/bin/thrall", "mcp", "fetch", "--", "uvx", "mcp-server-fetch"]`.
README "The tool document" renders it byte for byte as it renders the other
two (the existing test holds them together) and carries one paragraph: the
web tool is a pinned server and not a thrall feature; `uvx` is the operator's
to install (the image's alpine floor does not carry it — README "The image":
"what can run on it is still the operator's problem"); a web SEARCH server
needs a credential and is the operator's own pin.

Tests: the fake server fixture from child 1 grown to answer `tools/list` with
two tools, one carrying annotations; the printed array round-trips through
`config::read` as a valid document; the stderr commentary names the annotated
tool; the example document still reads and still matches the README.

Module map row: `src/mcp/pin.rs`.