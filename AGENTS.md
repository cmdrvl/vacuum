# AGENTS.md — vacuum

> Guidelines for AI coding agents working in this Rust codebase.

---

## vacuum — What This Project Does

`vacuum` walks directories and emits a deterministic, sorted JSONL manifest of every file — path, size, mtime, extension, and MIME type.

Pipeline position:

```
vacuum → hash → fingerprint → lock → pack
```

### Quick Reference

```bash
# Core pipeline
vacuum /data/dec > manifest.jsonl

# With filters
vacuum /data/dec --include "*.csv" --exclude ".DS_Store" > manifest.jsonl

# Quality gate
cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test
```

### Source of Truth

- **Spec:** [`docs/PLAN.md`](./docs/PLAN.md) — behavior must follow this document.
- Do not invent behavior not present in the plan.

### Key Files (planned)

| File | Purpose |
|------|---------|
| `src/main.rs` | CLI entry + exit code mapping |
| `src/lib.rs` | Orchestration flow |
| `src/cli/` | Argument parsing + witness subcommands |
| `src/scan/` | Directory walking, file stat, record construction |
| `src/filter/` | Include/exclude glob matching |
| `src/mime/` | Extension-based MIME type lookup |
| `src/output/` | Deterministic sorted JSONL emission |
| `src/refusal/` | Refusal envelope and codes |
| `src/witness/` | Witness append/query behavior |
| `operator.json` | Machine-readable operator contract |

---

## RULE 0 — USER OVERRIDE

If the user gives a direct instruction, follow it even if it conflicts with defaults in this file.

---

## Output Contract (Critical)

`vacuum` is a **directory scanner** that emits JSONL to stdout:

- Normal path emits sorted JSONL records to stdout (one per file).
- Refusal path emits one refusal JSON envelope to stdout.
- No human-report mode on stdout — pure JSONL only.

| Exit | Meaning |
|------|---------|
| `0` | `SCAN_COMPLETE` — all roots enumerated (some files may be `_skipped`) |
| `2` | `REFUSAL` — root-level failure (not found, permission denied, I/O error) |

Note: there is **no exit code 1** — per-file failures produce `_skipped` records, not partial outcomes.

---

## Core Invariants (Do Not Break)

### 1. Deterministic sorted output

- Records sorted by `(relative_path, root)` byte-order.
- Same directory always produces identical output.
- All records collected in memory before emission (ordering requires it).

### 2. Record format

- `version` must be `"vacuum.v0"`.
- Required fields: `path`, `relative_path`, `root`, `size`, `mtime`, `extension`, `mime_guess`, `tool_versions`.
- `tool_versions` must include `{ "vacuum": "<semver>" }`.

### 3. `_skipped` semantics

- Files that can't be stat'd produce `_skipped: true` records with `_warnings` array.
- Skipped records have `size: null` and `mtime: null`.
- Skipped files do NOT prevent `SCAN_COMPLETE` (exit 0).

### 4. Multi-root interleaving

- Multiple roots produce a single sorted manifest.
- `root` field distinguishes origin.
- `relative_path` is the sort key, `root` is the tiebreaker.

### 5. Refusal boundary

- Root not found / not readable / I/O preventing scan start are refusals (exit 2).
- Fail-fast on first failing root when multiple roots provided.
- Per-file stat failures are `_skipped` records, not refusals.

### 6. Witness parity

Ambient witness semantics must match spine conventions:
- Append by default to `$EPISTEMIC_WITNESS` or `~/.epistemic/witness.jsonl`.
- `--no-witness` opt-out.
- Witness failures do not mutate domain outcome semantics (non-fatal).
- Witness query subcommands supported (`query`, `last`, `count`).

---

## Toolchain

- **Language:** Rust, Cargo only.
- **Unsafe code:** forbidden in binary (`#![forbid(unsafe_code)]`).
- **Dependencies:** explicit versions, small and pinned.

Release profile:

```toml
[profile.release]
opt-level = "z"
lto = true
codegen-units = 1
panic = "abort"
strip = true
```

---

## Quality Gate

Run after any substantive change:

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

### Minimum Coverage Areas

- Deterministic sorted output (same dir → same JSONL),
- Include/exclude glob filtering,
- Multi-root interleaving and sort order,
- `_skipped` record handling (permission denied, broken symlinks),
- Refusal paths (root not found, root not readable),
- Outcome and exit-code routing,
- Witness append/query behavior,
- MIME type lookup for known extensions,
- E2E spine compatibility (`vacuum → hash → lock`).

---

## Git and Release

- **Primary branch:** `main`.
- **`master`** exists for legacy URL compatibility — keep synced: `git push origin main:master`.
- Bump `Cargo.toml` semver appropriately on release.
- Sync `Cargo.lock` before release workflows that use `--locked`.

---

## Editing Rules

- **No file deletion** without explicit written user permission.
- **No destructive git commands** (`reset --hard`, `clean -fd`, `rm -rf`, force push) without explicit authorization.
- **No scripted mass edits** — make intentional, reviewable changes.
- **No file proliferation** — edit existing files; create new files only for real new functionality.
- **No surprise behavior** — do not invent behavior not in `docs/PLAN.md`.
- **No backwards-compatibility shims** unless explicitly requested.

---

## Beads (`br`) Workflow

Use Beads as source of truth for task state.

```bash
br ready              # Show unblocked ready work
br list --status=open # All open issues
br show <id>          # Full issue details
br update <id> --status=in_progress
br close <id> --reason "Completed"
br sync --flush-only  # Export to JSONL (no git ops)
```

Pick unblocked beads. Mark in-progress before coding. Close with validation evidence.

---

## Multi-Agent Coordination

### File Reservation Policy (strict)

When multiple agents work concurrently, reserve only exact files you are actively editing.

Allowed: `src/scan/walk.rs`, `tests/scan_suite.rs`, `README.md`
Forbidden: `src/**`, `src/scan/`, `tests/**`

Release reservations as soon as your edits are complete.

### Agent Mail

When Agent Mail is available:
- Register identity in this project.
- Reserve only specific files you are actively editing — never entire directories.
- Send start/finish updates per bead.
- Poll inbox regularly and acknowledge `ack_required` messages promptly.
- Release reservations when done.

---

## Session Completion

Before ending a session:

1. Run quality gate (`fmt` + `clippy` + `test`).
2. Confirm docs/spec alignment for behavior changes.
3. Commit with precise message.
4. Push `main` and sync `master`.
5. Summarize: what changed, what was validated, remaining risks.
