#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
BIN="${BIN:-$ROOT_DIR/target/debug/vacuum}"

"$BIN" --robot-triage | jq -e '
  .schema_version == "vacuum.doctor.triage.v1"
  and .capabilities.agent_surfaces.machine_discovery == "vacuum capabilities --json"
  and .capabilities.agent_surfaces.robot_triage == "vacuum --robot-triage"
' >/dev/null

"$BIN" capabilities --json | jq -e '
  .schema_version == "vacuum.doctor.capabilities.v1"
  and .agent_surfaces.agent_guide == "vacuum robot-docs guide"
' >/dev/null

"$BIN" robot-docs guide | grep -F "vacuum --json <ROOT>..." >/dev/null
"$BIN" --json "$ROOT_DIR/tests/fixtures/simple" --no-witness | jq -e 'select(.version == "vacuum.v0")' >/dev/null
