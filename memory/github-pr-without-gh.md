---
name: github-pr-without-gh
description: How PRs are opened for priyansh1210/cursora without the gh CLI
metadata:
  node_type: memory
  type: reference
  originSessionId: 13cf765a-4fc3-4f3c-83f4-17c71dfbf14b
  modified: 2026-10-14T03:46:19.505Z
---

`gh` isn't installed on the laptop ([[dev-machines]]); on the other PC, check first. PRs for https://github.com/priyansh1210/cursora are opened with the GitHub REST API: get the token with `printf 'protocol=https\nhost=github.com\n\n' | git credential fill` (never print it), then `curl -X POST https://api.github.com/repos/priyansh1210/cursora/pulls` with a JSON body built by node from a markdown file. CI runs are listed at `/repos/priyansh1210/cursora/actions/runs`. One branch and PR per spec (`spec-NN-<name>`). Related: [[no-ai-attribution-in-commits]].
