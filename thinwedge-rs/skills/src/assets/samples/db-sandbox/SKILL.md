---
name: "db-sandbox"
description: "Use when a task touches database state, migrations, schema changes, dbt/model changes, RLS or policy changes, backfills, SQL-heavy finance metrics, or production-like database tests. Creates or uses an isolated Ardent Postgres branch and passes only the branch DATABASE_URL to the agent workflow."
metadata:
  short-description: "Safe Ardent database sandboxing"
---

# DB Sandbox

Use this skill only for tasks that need realistic database state. Do not load it for normal chat, file-only analysis, or spreadsheet-only transformations.

## When To Use

Use an Ardent branch for:

- schema migrations
- data migrations and backfills
- dbt or SQL model changes
- row-level security or policy changes
- SQL-heavy finance metrics
- production-like database tests

## Safety Rules

- Never request, print, store, or pass the source database URL to an agent.
- Agents receive only `DATABASE_URL` for the Ardent branch.
- Treat connector creation, production source wiring, RDS parameter changes, security-group changes, DB reboots, and DB user creation as approval-gated operations.
- Branch create, branch info, branch health checks, and branch delete can run without repeated prompts after the Ardent connector is configured.
- Delete task branches when the task is done unless the user asks to keep one for review.

## Default Workflow

1. Check readiness with the probe scripts before changing CLI code or running a full rebuild.
2. Create a task branch using the configured connector.
3. Export only the branch connection URL as `DATABASE_URL` for migrations/tests.
4. Run the database task and validations against the branch.
5. Report migration/test results and delete the branch or mark it for review.

## Commands

Prefer the ThinWedge wrapper commands when available:

```bash
thinwedge ardent status
thinwedge ardent branch create
thinwedge ardent branch delete
```

Until wrapper commands are wired, use the scripts under `scripts/probes/` to prove auth, connector readiness, and branch lifecycle behavior.
