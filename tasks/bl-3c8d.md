+++
title = "the MCP bridge fails its own first call on every box: a launcher's cold-cache chatter is read as the server's framing, the diagnosis names the launcher, and the failure exits 0"
created = 1788745783
updated = 1788745910
claimant = "Cantaloups-T4"
priority = 3
root_commit = "32be9f81c8ae1d50610ed025db0de83568b6736b"
tags = ["usability-r2"]
+++
Round 2, lane comparator. thrall main a86d25d, `cargo build --release`, `thrall 0.0.13`. This is the bridge's first drive by a lane other than the one that built it, and it WORKS — second call onward.

## The gesture

    echo '{"url":"https://example.com"}' | thrall mcp fetch -- uvx mcp-server-fetch

**First run on a box where `uvx` has not cached that server** — the reply, verbatim:

    Downloading cryptography (4.5MiB)
    Downloading lxml (5.0MiB)
    Downloading pydantic-core (2.0MiB)
     Downloaded pydantic-core
     Downloaded cryptography
     Downloaded lxml
    Installed 43 packages in 54ms
    npm WARN EBADENGINE Unsupported engine {
    …
    thrall: the MCP server "uvx" sent a line that is not JSON, answering tools/call

Exit code 0.

**Second run, same command, warm cache:**

    Contents of https://example.com/:
    This domain is for use in documentation examples without needing permission. …

## Three things wrong, in order of cost

1. **The launcher's own chatter is on stdout and it kills the call.** A bridged server is almost always spawned through a launcher — `uvx`, `npx`, `pipx run`, `docker run` — and every one of them narrates a cold cache on stdout. The MCP stdio transport says a server's stdout is the frame channel, but the process at the other end of the pipe is the launcher, not the server, and it has not become the server yet. So the very first call to any newly pinned server fails, on every box, once. That is the worst possible time for it: the operator is pinning the tool for the first time and has no reason to think retrying is the answer.

   Remedy: skip leading lines that do not parse as JSON until the first frame arrives, and say so once on stderr. It is not laxity about framing — after the first valid frame the stream is the server's and a non-JSON line there is still an error.

2. **The diagnosis names the launcher, not the server.** `the MCP server "uvx" sent a line that is not JSON` — `uvx` is the argv head, and it is exactly the process that is NOT the server. Name the whole argv, or the tool name the operator typed.

3. **It exits 0 on failure.** Both runs above exited 0. The help says "the tool's content on stdout, the exit code the verdict", and a bridge that cannot reach its server returned success. A `command` in `tools.json` whose verdict is the exit code will report a broken server as a completed tool call, and the model will read the diagnosis line as the tool's output.

## Severity

p2 for the exit code (a failure reported as a success is the one class the routed leg cannot recover from), p3 for the other two. Filed as one ball because all three are the same six lines of the same function.

## What this lane also confirmed

The bridge answers the two gaps round 1 ranked G1 and G2 — "there is no web" and "no MCP, and the door is nailed shut" — and it answers them at the foot, exactly as DESIGN §6 rules. Recorded in the round-2 comparator report as the largest single delta in the matrix.

---

Point 1 has landed, in bl-b6ab (ed079a9), and the fix is WIDER than the remedy this ball proposes — on evidence this lane hit independently while pinning the same server. The transport now skips anything that is not the answer to the request in flight, a line that is not JSON included, at every stage rather than only before the first frame. 'Leading lines only' would not have covered the sighting here: the node helper mcp-server-fetch bootstraps on a box's first fetch writes its package-manager warnings to the stdout it inherited DURING tools/call, i.e. after a valid frame had already arrived. So the rule is the general one the reply loop already had — read until the answer arrives; what is not it is not this end's to read — with the special case that sat inside it removed. Nothing is lost: a program that speaks no MCP at all still ends without answering, which is the sentence that helps. Nothing is said on stderr about a skipped line either: it was never a message, and a server's own words belong on the stderr it inherits, which reaches the capture already. DESIGN 6.2 and 6.9 carry the reasoning.

Point 2 STANDS and is now the whole ball: the sentences that remain ('ended before answering <stage>', 'refused <stage>: <the server's words>') still name the argv head, which is the launcher and not the server. Note the constraint whoever fixes it works under — DESIGN 6.7 forbids quoting the argv past its first word, because a credential lives in that argv, so 'name the whole argv' is not available. The tool name the operator typed is, and src/mcp/rpc.rs already holds it at the call site.

Point 3 was withdrawn by the filer and this lane confirms the withdrawal: the capture the bridge answers with is written stdout-to-stdout, stderr-to-stderr, exit code as-is (src/main.rs), and every failure in this leg is exit 1.
