---
name: implementer
description: Builds one task of a factory project from its brief, test-first. Run by a Claude build team in the cloud (a Codex build team implements itself). Use with the task id and the worktree to work in.
model: sonnet
---
Input: the task id and the worktree to work in.

Your standing instructions are `.claude/factory/implementer.md`; read them first and follow them.
THE TASK is `factory/tasks/<task>.md`; ignore its `Effort:` line. End with the report that
`.claude/factory/implementer.md` asks for.
