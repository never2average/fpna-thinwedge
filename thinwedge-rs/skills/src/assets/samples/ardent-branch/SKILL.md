---
name: "ardent-branch"
description: "Use when a task needs a disposable Ardent Postgres branch for migrations, schema changes, dbt/model work, RLS or policy changes, backfills, SQL-heavy finance metrics, or production-like database tests. Creates or uses an isolated branch and passes only the branch DATABASE_URL to the agent workflow."
metadata:
  short-description: "Ephemeral Ardent branch workflow"
---

# Ardent Branch

Use this skill only for work that needs a realistic database branch. Do not load it for normal chat, file-only analysis, or spreadsheet-only transformations.

## Safety Rules

- Never request, print, store, or pass the source database URL to an agent.
- Agents receive only `DATABASE_URL` for the Ardent branch.
- Treat connector creation, production source wiring, managed-vs-BYOC data-plane selection, RDS parameter changes, security-group changes, DB reboots, and DB user creation as approval-gated operations.
- After an Ardent connector is configured, selecting a connector and creating, inspecting, or deleting task branches can run without repeated prompts.
- Delete task branches when the task is done unless the user asks to keep one for review.

## Workflow

1. Run `thinwedge ardent status` or the `scripts/probes/` readiness checks before relying on the sandbox.
2. Create a task branch with `thinwedge ardent branch create --print-env`.
3. Export only the branch connection URL as `DATABASE_URL` for migrations/tests.
4. Run the database task and validations against the branch.
5. Report results and delete the branch or mark it for review.

## Commands

```bash
thinwedge ardent status
thinwedge ardent branch create --print-env
thinwedge ardent branch delete <branch-name>
```

Use the broader `db-sandbox` skill when you need the full readiness and setup workflow, including AWS billing/DB Ops checks and connector readiness.
