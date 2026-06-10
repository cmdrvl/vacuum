# vacuum Agent Ergonomics Playbook

## Canonical Agent Commands

```bash
vacuum --robot-triage
vacuum capabilities --json
vacuum robot-docs guide
vacuum --json <ROOT>... --no-witness
```

## Applied Improvements

1. `vacuum --robot-triage` returns health, capabilities, contracts, and recommended actions in one JSON payload.
2. `vacuum capabilities --json` gives agents the command surface without entering doctor mode.
3. `vacuum robot-docs guide` prints a compact in-tool agent guide.
4. `--json` is accepted for scans as explicit machine-output intent.
5. Root refusal envelopes now carry actionable `refusal.next_command` values.
6. `vacuum doctor --fix` exits `2` with safe read-only alternatives instead of generic clap output.

## Regression Commands

```bash
bash agent_ergonomics_audit/audit/regression_tests/R-001__agent_discovery.test.sh
bash agent_ergonomics_audit/audit/regression_tests/R-002__refusal_guidance.test.sh
```
