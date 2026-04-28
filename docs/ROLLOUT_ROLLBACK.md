# Rollout and Rollback Instructions

This guide defines a safe rollout plan for `isolate` changes and a rollback path for each phase.

## Goals

- Ship changes in small, reversible increments.
- Detect regressions early with explicit gates.
- Provide a deterministic rollback path at every step.

## Pre-Rollout Checklist

Run and record these checks before any rollout step:

```bash
moon run :quick
moon run :test
moon run :ci
isolate --version
```

Confirm:

- No local panic/unwrap regressions.
- CI green on the release commit.
- Release owner and rollback owner assigned.

## Phased Rollout Plan

### Phase 0 - Dry Run in Local/CI

Actions:

- Run the exact release candidate commit in CI and one local operator environment.
- Verify critical commands with JSON output paths used by automation.

Success criteria:

- CI green (`:ci`).
- No command contract drift in manual smoke commands.

Rollback:

- Stop promotion.
- Fix on current branch.
- Re-run Phase 0.

### Phase 1 - Limited Operator Rollout

Actions:

- Deploy to a small set of operators/agents.
- Monitor command failures and system health.

Suggested checks:

```bash
swarm status
swarm monitor --view failures
```

Success criteria:

- Error rate stays at baseline.

Rollback:

- Revert to previous tagged commit.
- Restart affected operator sessions.
- Validate state health after downgrade.

### Phase 2 - Broad Rollout

Actions:

- Promote to all operators after stable Phase 1 window.
- Continue monitoring at fixed intervals during the first hour.

Success criteria:

- No P0/P1 incidents.
- Normal throughput and agent completion behavior.

Rollback:

- Perform version rollback to last known-good build.
- Freeze new deployments until root cause is documented.

## Rollback Procedure (Operational)

1. Identify last known-good commit/tag.
2. Deploy that artifact to all affected runners.
3. Verify health with:

```bash
isolate --version
swarm status
```

4. Confirm critical command paths used by automation.
5. Open remediation bead with incident notes and reproduction details.

## Failure Triggers That Require Immediate Rollback

- Command contract breaks (`--json` structure drift, non-deterministic exits).
- Lock contention spikes.
- Swarm agent failure cascade.
- Data corruption indicators in workspace/session state.

## Communications Template

Use this format in team updates:

- `Change`: <commit/tag>
- `Phase`: <0|1|2>
- `Start`: <timestamp>
- `Health`: <green|yellow|red>
- `Decision`: <continue|pause|rollback>
- `Owner`: <name>

## Post-Rollout Closeout

- Record metrics and incidents.
- Update `docs/ERROR_TROUBLESHOOTING.md` and failure taxonomy if new failure classes were observed.
- Close rollout bead only after a stable monitoring window.
