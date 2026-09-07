+++
title = "acceptance: mcp-server-fetch round-trips through a yog engine as a pinned tool — the bridge's first real server, and the web tool"
created = 1788744507
updated = 1788746117
claimant = "Cantaloups-T4"
priority = 3
root_commit = "32be9f81c8ae1d50610ed025db0de83568b6736b"
tags = ["usability-r2"]

[[blockers]]
id = "bl-e104"
on = "claim"

[[blockers]]
id = "bl-b6ab"
on = "claim"
+++
Child 4 of the MCP bridge design (DESIGN §6, bl-3b03). One lane-day, drive
shaped; the next usability round re-drives it. Needs the bridge verb
(bl-e104), the pin verb and example entry (bl-b6ab) and yog's name-keyed
policy row (yog bl-b65d — a cross-repo edge the store cannot carry; check it
landed before claiming).

WHAT IT PROVES — the empirical half litany's DESIGN_MCP_BRIDGE.md §9 says has
never existed: one bridge, one real server, one model reading a page.

1. On a scratch root (never the operator's ambient world), a yog engine with
   a foot enrolled; `uvx` present on the foot's box.
2. `thrall mcp pin -- uvx mcp-server-fetch` prints the entries; the operator
   pastes `fetch` into `tools.json` (the shipped example entry is that paste,
   byte for byte); `thrall run` advertises it; `/clients` shows it on the box.
3. A conversation loads `<foot>_fetch` through the `clients` tool and calls
   it on a URL. The FIRST call is held — opaque, yog bl-72bd — and the hold
   text names the `rules:` row to write. Record the sentence verbatim.
4. The operator writes `rules:` / `  <foot>_fetch: open-world` into the
   workspace's `capability.yaml`; the next call passes; the capture is the
   page as markdown; the model answers a question from it.
5. `git -C ~/dev/litany diff` is empty and `git -C ~/dev/yog diff` is empty:
   the whole exercise lands in the foot's document and the workspace's policy.

Evidence: the drive log (kept outside the checkout per yog QUALITY §3), the
verbatim hold sentence, the capture head, and the wall time of one fetch —
the number that decides whether DESIGN §6.4's deferred warm-server option is
ever paid for. If a step fails, file the defect in the repo that owns it and
leave this ball open naming it.

---

DRIVEN as far as it goes today; left open on yog bl-b65d, which is NOT on yog main (git log main | grep b65d is empty).

WHAT PASSED, on a scratch root with a yog main engine (PROTOCOL 16) and this crate's main (PROTOCOL 16):
- wire-certs, bare yog listening, /prepare dir, /enroll <name> foot, four files into the foot's wire/workspaces/<ws>/.
- `thrall mcp pin -- uvx mcp-server-fetch` on a COLD uv cache: exit 0, one complete fetch entry on stdout, the launcher's download chatter on stderr and never on the frame channel. First cold-cache drive since bl-b6ab widened the skip; it holds.
- the entry pasted into tools.json, `thrall run` advertises it, /clients shows the box present with fetch and its schema.
- /invoke <foot> fetch {url} routes it: the foot's process tree during the call is thrall run -> thrall mcp fetch -> uv tool uvx -> mcp-server-fetch, and /capture <handle> answers exit 0 with the page as markdown. The round trip is real.
- git diff empty in both litany and yog: the whole exercise landed in the foot's document.

WALL TIME, the number DESIGN 6.4 asked for. One `thrall mcp fetch` invocation, spawn to capture: 20.0s on a cold uv cache (the pin, once per box), 4.2s on the first fetch while the server bootstraps its node helper, then 2.8s and 1.3s warm. Steady-state cost of the per-invocation server start is ~1.3s.

WHAT IS BLOCKED. Step 4 needs bl-b65d. The hold is real -- feeding the control a <foot>_fetch request answers verdict hold, reason: classified opaque (... is not a tool this control implements and its input carries no command line, so what it reaches cannot be read -- held rather than passed). Two things follow: that sentence does not name the rules: row as the way out, because naming it is bl-b65d's half; and the row would not be read anyway, since yog's src/control/policy.rs parses a rules: item as "program [words]: class" and matches it against a command line, so a routed tool name matches nothing. The only way past a held opaque call today is table: / opaque: pass, the workspace-wide blanket DESIGN 6.6 explicitly refuses.

The model half was not driven: it ends at the same hold, and spending on a model to reach a known block buys nothing.

Re-drive after bl-b65d: steps 1-3 and 5 reproduce from the drive log (kept beside its evidence outside the checkout, per yog QUALITY 3); what remains is the rules: row, the hold sentence naming it, the next call passing, and one real conversation over it.
