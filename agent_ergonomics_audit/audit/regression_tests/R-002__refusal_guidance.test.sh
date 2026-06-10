#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
BIN="${BIN:-$ROOT_DIR/target/debug/vacuum}"

set +e
empty_output="$("$BIN" --no-witness)"
empty_status=$?
missing_output="$("$BIN" /definitely-missing-vacuum-root-audit --no-witness)"
missing_status=$?
fix_stderr="$("$BIN" doctor --fix 2>&1 >/dev/null)"
fix_status=$?
set -e

test "$empty_status" -eq 2
test "$missing_status" -eq 2
test "$fix_status" -eq 2

printf '%s\n' "$empty_output" | jq -e '.refusal.next_command == "vacuum ."' >/dev/null
printf '%s\n' "$missing_output" | jq -e '.refusal.next_command == "ls -la '\''/'\''"' >/dev/null
printf '%s\n' "$fix_stderr" | grep -F "vacuum doctor --robot-triage" >/dev/null
printf '%s\n' "$fix_stderr" | grep -F "vacuum capabilities --json" >/dev/null
