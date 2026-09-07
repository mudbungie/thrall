+++
title = "acceptance: mcp-server-fetch round-trips through a yog engine as a pinned tool — the bridge's first real server, and the web tool"
created = 1788744507
updated = 1788746038
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