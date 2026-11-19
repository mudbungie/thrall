+++
title = "A refused fast-forward in the advisories workflow is green only when main moved"
created = 1795071859
updated = 1795071859
claimant = "Knuckled-T"
root_commit = "32be9f81c8ae1d50610ed025db0de83568b6736b"
+++
The landing step forgave every refused push of the mend branch to main. It now fetches main after a refusal: if the tip of main is still an ancestor of the branch, main did not move and the push was refused for some other reason, so the step says that is for a hand and exits red. Only when main really moved does it print its line and end green with nothing landed.