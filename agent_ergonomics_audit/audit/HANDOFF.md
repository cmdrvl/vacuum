# vacuum Agent Ergonomics Handoff

Pass 1 completed in full mode on 2026-06-10.

## Completed

- Added top-level agent discovery commands: `--robot-triage`, `capabilities --json`, and `robot-docs guide`.
- Accepted `--json` for scans while preserving pure JSONL stdout.
- Added actionable `refusal.next_command` values for root refusals.
- Made unavailable `doctor --fix` name safe read-only alternatives.
- Updated README, plan, and `operator.json`.
- Added focused Rust regression tests and audit shell regressions.

## Validation

- `cargo fmt --check`
- `cargo clippy --all-targets -- -D warnings`
- `cargo test`
- Intent corpus: 116 total, 116 useful hints, 0 useless errors, 0 silent failures

## Follow-Up

- After release assets are available, verify the Homebrew-installed binary with the audit regression scripts.
- The skill preflight reported missing `flock` on macOS; this was non-blocking for the single-agent pass.
