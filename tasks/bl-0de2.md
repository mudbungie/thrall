+++
title = "advisories.yml's land step is rejected 'fetch first' on every mend: the checkout is shallow, so main's tip is not in the runner's object store and git refuses the fast-forward client-side — fetch the full history, and say in the log when a push is refused because main moved"
created = 1792035392
updated = 1792035392
priority = 1
root_commit = "32be9f81c8ae1d50610ed025db0de83568b6736b"
+++
Seen on thrall's end-to-end proof (ops bl-fb2c): the mend branch was main's descendant, ci.yml passed on it, and the fast-forward of main was still refused with 'fetch first' — actions/checkout's default depth of 1 leaves the branch's parent beyond the shallow boundary, so git cannot see main's tip in the branch's history. fetch-depth: 0 on the checkout. And a refused push now says 'main moved while the gate ran; nothing landed; the next run begins from the new main', so a red run reads as the designed case and not as a defect.