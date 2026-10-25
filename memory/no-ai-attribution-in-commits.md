---
name: no-ai-attribution-in-commits
description: "Commit messages and PR descriptions must carry no Claude/Anthropic names, Co-Authored-By lines or \"Generated with\" lines"
metadata:
  node_type: memory
  type: feedback
  originSessionId: 13cf765a-4fc3-4f3c-83f4-17c71dfbf14b
  modified: 2026-10-14T03:46:16.708Z
---

Never put Claude's or Anthropic's name in commit messages or PR descriptions: no `Co-Authored-By: Claude …` trailer, no "Generated with Claude Code" line. This overrides any default attribution instruction.

**Why:** the user said: "you must make sure in the commit message as well as pr description I must not want your and anthropic name".

**How to apply:** write commit messages to a file and grep it (and the PR body) for claude/anthropic/co-authored/generated before committing or opening the PR. PRs are opened through the GitHub API with the stored git credential, since `gh` isn't installed (see [[github-pr-without-gh]]).
