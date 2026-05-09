# ThinWedge Probe Scripts

These scripts validate external integration contracts before the Rust CLI is rebuilt. They are intentionally bottom-up so AWS billing, AWS DB Ops, RDS readiness, Ardent auth, connector readiness, and branch lifecycle can be tested independently.

All probes support `--dry-run` for cheap syntax and wiring checks. Live runs require the relevant AWS and Ardent credentials in the current shell. Set `THINWEDGE_DB_SECRET_ID` or `THINWEDGE_DB_SSM_PARAMETER` to verify a specific DB connection secret. RDS readiness live runs require TCP reachability from the current host and `THINWEDGE_DB_ROLE_DATABASE_URL` for the DB setup role check.

```bash
scripts/probes/check-aws-billing.sh --dry-run
scripts/probes/check-aws-db-ops.sh --dry-run
scripts/probes/check-rds-postgres-readiness.sh --dry-run
scripts/probes/check-ardent-auth.sh --dry-run
scripts/probes/check-ardent-connector.sh --dry-run
scripts/probes/check-db-sandbox-readiness.sh --dry-run
```

Live validation expects the shell to already have:

- an AWS CLI on `PATH`
- a billing AWS identity with STS, Cost Explorer, CUR, Budgets, and IAM account-summary read access, via `THINWEDGE_BILLING_AWS_PROFILE` or `AWS_PROFILE`
- a DB Ops AWS identity, via `THINWEDGE_DB_OPS_AWS_PROFILE` or `AWS_PROFILE`
- `nc` for RDS TCP reachability checks
- `psql` plus `THINWEDGE_DB_ROLE_DATABASE_URL` for DB setup role validation
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
scripts/probes/check-ardent-auth.sh
scripts/probes/check-ardent-connector.sh --connector <ardent-connector-name>
scripts/probes/check-db-sandbox-readiness.sh
```

The RDS readiness probe fails live unless it can prove network reachability and
that the DB setup role has `rolreplication` or superuser capability. Use
`--skip-network-check` or `--skip-db-role-check` only for a narrow metadata-only
inspection, not for production readiness sign-off.

Mutation-gated checks require explicit opt-in:

```bash
TW_PROBE_ALLOW_MUTATION=1 scripts/probes/check-ardent-branch-lifecycle.sh
```

The branch lifecycle probe creates and deletes an Ardent branch. Connector creation is also mutation-gated because it wires Ardent to a source database. Source database URLs must be provided only through environment variables and are never printed by the scripts.
