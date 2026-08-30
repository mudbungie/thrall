+++
title = "the tool config surface: one document, two readings"
created = 1787977355
updated = 1788060703
claimant = "OrderWright"
priority = 2
root_commit = "32be9f81c8ae1d50610ed025db0de83568b6736b"

[[blockers]]
id = "bl-e5ba"
on = "claim"
+++
The operator-authored document that says what this box offers, and the local gate on what is enabled (yog REMOTE §5.2, "the tool host's own config").

One JSON document, out of world because it describes *this machine*, sibling of the entry directory and of any generated state — for the same reason the key material is.

```json
[{"name": "Bash",
  "description": "run a command in a shell",
  "input_schema": {"type": "object",
                   "properties": {"command": {"type": "string"}},
                   "required": ["command"]},
  "command": ["/usr/local/libexec/thrall-tools/bash-tool"],
  "cwd": "/srv/work"}]
```

- The first three keys **are** the advertised element, verbatim; `command` and the optional `cwd` are the local half. The advertisement is the projection that drops the local half — **one document, two readings**, so what a host offers and what it can actually run cannot drift. That is the whole reason the config is not a second list beside the advertisement.
- **`name` is a single path component.** A name carrying a separator is a name that addresses a filesystem.
- **`input_schema` is JSON Schema, verbatim.** Neither validated nor rewritten: it is this host's statement to a model, and narrowing it would be inventing a contract nobody owns. JSON rather than TOML for exactly this reason — any other syntax makes the operator transcribe it.
- **`command` is an argv, spawned directly.** No shell, no interpolation of the invocation's input into it. A shell would make the declared schema advisory and turn an operator's config into a command-injection surface for anything the model can type.
- **A name collision inside one host's set declines loudly**, naming the token. A collision across hosts is legal and ordinary and is disambiguated at the far end's load act.
- **Local config gates what is enabled.** This document is the gate: a tool absent from it is a tool this box does not have. Server-side adjudication is unchanged and fails closed, and stacks on top.
- No version key, no enable flag, no per-workspace list — each would be a fact stored on one side and checkable on neither.