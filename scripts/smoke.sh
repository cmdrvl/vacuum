#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

echo "[smoke] building release binary"
cargo build --release >/dev/null
BIN="$ROOT_DIR/target/release/vacuum"

echo "[smoke] checking --version"
VERSION_OUTPUT="$("$BIN" --version)"
if [[ "$VERSION_OUTPUT" != vacuum\ * ]]; then
  echo "expected version output, got: $VERSION_OUTPUT" >&2
  exit 1
fi

echo "[smoke] checking --describe"
DESCRIBE_OUTPUT="$("$BIN" --describe)"
if ! printf '%s\n' "$DESCRIBE_OUTPUT" | grep -Eq '"schema_version"[[:space:]]*:[[:space:]]*"operator.v0"'; then
  echo "describe output missing operator schema_version" >&2
  exit 1
fi

echo "[smoke] checking --schema"
SCHEMA_OUTPUT="$("$BIN" --schema)"
if ! printf '%s\n' "$SCHEMA_OUTPUT" | grep -Eq '"title"[[:space:]]*:[[:space:]]*"vacuum.v0"'; then
  echo "schema output missing vacuum.v0 title" >&2
  exit 1
fi

echo "[smoke] scanning fixture output"
SCAN_OUTPUT="$("$BIN" "$ROOT_DIR/tests/fixtures/simple")"
if ! printf '%s\n' "$SCAN_OUTPUT" | grep -Eq '"version"[[:space:]]*:[[:space:]]*"vacuum.v0"'; then
  echo "scan output missing manifest version" >&2
  exit 1
fi

echo "[smoke] checking refusal exit code"
set +e
REFUSAL_OUTPUT="$("$BIN" "$ROOT_DIR/tests/fixtures/does-not-exist" 2>/dev/null)"
REFUSAL_EXIT=$?
set -e
if [[ $REFUSAL_EXIT -ne 2 ]]; then
  echo "expected refusal exit code 2, got $REFUSAL_EXIT" >&2
  exit 1
fi
if ! printf '%s\n' "$REFUSAL_OUTPUT" | grep -Eq '"outcome"[[:space:]]*:[[:space:]]*"REFUSAL"'; then
  echo "refusal output missing REFUSAL outcome" >&2
  exit 1
fi

echo "[smoke] success"
