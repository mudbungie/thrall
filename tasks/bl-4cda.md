+++
title = "the executor and the capture: run it, transcode once, answer"
created = 1787977360
updated = 1788060885
claimant = "OrderWright"
priority = 2
root_commit = "32be9f81c8ae1d50610ed025db0de83568b6736b"

[[blockers]]
id = "bl-a2ea"
on = "claim"
+++
The execution half of the foot — what happens between `invocations` handing over a call and `complete` answering it (yog REMOTE §5.3).

- **The invocation reaches the command exactly as a local tool contract already delivers one**: the `tool_use.input` JSON on stdin, bytes on stdout, the exit code the verdict. So a foot's tool executable is the same kind of program a local pool tool is, and the capture that comes back is the same three facts.
- **A capture is text, and the transcode happens once.** A capture ends as a model's tool result, and a model's message is text — so the executor transcodes its child's bytes at the one place bytes stop being bytes, and nothing downstream carries an encoding case. A tool whose output is not UTF-8 loses exactly the bytes no string can name.
- **Two deadlines, measuring different things.** The foot's own bound terminates the child (SIGTERM then SIGKILL) and answers with a sentence; the asking side's longer patience stands behind it for the case where the whole foot process went away. Neither is a knob: an engine that has not answered is down, and a tool that has not answered is working.
- **A staleness refusal is in band.** A tool the config no longer carries is refused at the call, which is the correction path the far end relies on for a definition it froze at load time.
- **Containment honesty.** Execution happens on a machine the adjudicator cannot inspect. Adjudication at the far end judges the invocation and nothing more; any containment beyond that is whatever thrall enforces locally, and neither design may claim otherwise. Whatever thrall does enforce, it states plainly and does not overstate.

This ball is where thrall first spawns a child process, so it is also where the confinement rules land — see the gate-apparatus ball.