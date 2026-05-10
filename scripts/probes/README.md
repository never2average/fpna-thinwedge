# ThinWedge Probe Scripts

These scripts validate external integration contracts before the Rust CLI is rebuilt. They are intentionally bottom-up so AWS billing, AWS DB Ops, source database readiness, Ardent auth, connector readiness, and branch lifecycle can be tested independently.

All probes support `--dry-run` for cheap syntax and wiring checks. Live runs require the relevant AWS, Neon, database, and Ardent credentials in the current shell. Set `THINWEDGE_DB_SECRET_ID` or `THINWEDGE_DB_SSM_PARAMETER` to verify a specific DB connection secret. RDS readiness live runs require TCP reachability from the current host and `THINWEDGE_DB_ROLE_DATABASE_URL` for the DB setup role check. Generic Postgres, Supabase, and Neon readiness live runs require `THINWEDGE_ARDENT_SOURCE_DATABASE_URL`.

```bash
scripts/probes/check-aws-billing.sh --dry-run
scripts/probes/check-aws-db-ops.sh --dry-run
scripts/probes/check-rds-postgres-readiness.sh --dry-run
scripts/probes/check-postgres-source-readiness.sh --dry-run
scripts/probes/check-neon-postgres-readiness.sh --dry-run
scripts/probes/check-ardent-auth.sh --dry-run
scripts/probes/check-ardent-connector.sh --dry-run
scripts/probes/check-db-sandbox-readiness.sh --dry-run
```

Live validation expects the shell to already have:

- an AWS CLI on `PATH`
- a billing AWS identity with STS, Cost Explorer, CUR, Budgets, and IAM account-summary read access, via `THINWEDGE_BILLING_AWS_PROFILE` or `AWS_PROFILE`
- a DB Ops AWS identity, via `THINWEDGE_DB_OPS_AWS_PROFILE` or `AWS_PROFILE`
- `nc` for RDS TCP reachability checks
- `psql` plus `THINWEDGE_DB_ROLE_DATABASE_URL` for RDS DB setup role validation
- `psql` plus `THINWEDGE_ARDENT_SOURCE_DATABASE_URL` for generic Postgres, Supabase, and Neon source validation
- a Neon API key and project id in `THINWEDGE_NEON_API_KEY` and `THINWEDGE_NEON_PROJECT_ID` when `THINWEDGE_DB_SOURCE_PROVIDER=neon`
- an authenticated Ardent CLI from `ardent login`
- a selected Ardent project from `ardent project switch <name>`
- at least one Ardent connector; set `THINWEDGE_ARDENT_CONNECTOR` to verify the intended connector

Run the non-mutating live checks before trusting a finance DB sandbox setup:

```bash
cp scripts/probes/db-sandbox-readiness.env.example scripts/probes/db-sandbox-readiness.env
# Fill scripts/probes/db-sandbox-readiness.env with local values.
set -a
. scripts/probes/db-sandbox-readiness.env
set +a

scripts/probes/check-aws-billing.sh
scripts/probes/check-aws-db-ops.sh
scripts/probes/check-rds-postgres-readiness.sh --db-instance <rds-postgres-instance>
THINWEDGE_DB_SOURCE_PROVIDER=postgresql scripts/probes/check-postgres-source-readiness.sh
THINWEDGE_DB_SOURCE_PROVIDER=neon scripts/probes/check-neon-postgres-readiness.sh
scripts/probes/check-ardent-auth.sh
scripts/probes/check-ardent-connector.sh --connector <ardent-connector-name>
scripts/probes/check-db-sandbox-readiness.sh
```

The RDS readiness probe fails live unless it can prove network reachability and
that the DB setup role has `rolreplication` or superuser capability. Use
`--skip-network-check` or `--skip-db-role-check` only for a narrow metadata-only
inspection, not for production readiness sign-off.

The generic Postgres readiness probe checks network reachability, `wal_level`,
replication/superuser capability, and schema create privileges. The event-trigger
create/drop proof is mutation-gated because it briefly creates temporary source
database objects:

```bash
TW_PROBE_ALLOW_MUTATION=1 scripts/probes/check-postgres-source-readiness.sh
```

The Neon readiness probe first verifies the Neon API key, project id, logical
replication setting, and enabled read-write endpoint, then runs the generic
Postgres source checks. For full proof that the Neon API key can manage branches:

```bash
TW_PROBE_ALLOW_MUTATION=1 scripts/probes/check-neon-postgres-readiness.sh --include-api-branch-smoke
```

Ardent BYOC Neon connector creation uses:

```bash
TW_PROBE_ALLOW_MUTATION=1 scripts/probes/check-ardent-connector.sh --create --source-provider neon
```

If Ardent returns `snapshot_max_connections` during BYOC Neon setup, the local
Neon source can still be ready; that error is an Ardent server-side setup
failure. Keep the failed connector cleaned up and retry after Ardent fixes the
BYOC Neon path.

Mutation-gated checks require explicit opt-in:

```bash
TW_PROBE_ALLOW_MUTATION=1 scripts/probes/check-ardent-branch-lifecycle.sh
```

The branch lifecycle probe creates and deletes an Ardent branch. Connector creation is also mutation-gated because it wires Ardent to a source database. Source database URLs must be provided only through environment variables and are never printed by the scripts.
