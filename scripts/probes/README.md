# ThinWedge Probe Scripts

These scripts validate external integration contracts before the Rust CLI is rebuilt. They are intentionally bottom-up so AWS billing, AWS DB Ops, RDS readiness, Ardent auth, connector readiness, and branch lifecycle can be tested independently.

All probes support `--dry-run` for cheap syntax and wiring checks. Live runs require the relevant AWS and Ardent credentials in the current shell. Set `THINWEDGE_DB_SECRET_ID` or `THINWEDGE_DB_SSM_PARAMETER` to verify a specific DB connection secret. Set `THINWEDGE_CHECK_DB_NETWORK=1` to test TCP reachability from the current host to the selected RDS endpoint.

```bash
scripts/probes/check-aws-billing.sh --dry-run
scripts/probes/check-aws-db-ops.sh --dry-run
scripts/probes/check-rds-postgres-readiness.sh --dry-run
scripts/probes/check-ardent-auth.sh --dry-run
scripts/probes/check-ardent-connector.sh --dry-run
scripts/probes/check-db-sandbox-readiness.sh --dry-run
```

Mutation-gated checks require explicit opt-in:

```bash
TW_PROBE_ALLOW_MUTATION=1 scripts/probes/check-ardent-branch-lifecycle.sh
```

The branch lifecycle probe creates and deletes an Ardent branch. Connector creation is also mutation-gated because it wires Ardent to a source database. Source database URLs must be provided only through environment variables and are never printed by the scripts.
