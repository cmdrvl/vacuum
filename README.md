# vacuum

<div align="center">

[![CI](https://github.com/cmdrvl/vacuum/actions/workflows/ci.yml/badge.svg)](https://github.com/cmdrvl/vacuum/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![GitHub release](https://img.shields.io/github/v/release/cmdrvl/vacuum)](https://github.com/cmdrvl/vacuum/releases)

**Know what's there before you touch anything.**

```bash
brew install cmdrvl/tap/vacuum
```

</div>

---

You receive a data delivery. 47 files in a directory — CSVs, Excel workbooks, PDFs, a stray `.tmp` someone forgot to delete. Before you can hash them, fingerprint them, or lock them down, you need the answer to the simplest question in data: *what's actually there?*

**vacuum walks directories and emits a deterministic JSONL manifest — the same files in, the same manifest out, every time.** Each file gets a record with its path, size, modification time, and MIME type. No content reading, no transformation, no heuristics. Just enumeration — the reliable starting point for everything downstream.

### What makes this different

- **Deterministic by construction** — records sorted by `(relative_path, root)` byte-order. Same directory always produces byte-identical output.
- **Multi-root scanning** — scan `/data/q3` and `/data/q4` in one invocation; records from all roots interleave deterministically in a single manifest.
- **Nothing is silently dropped** — files that can't be stat'd produce `_skipped` records with warnings, not silent omissions.
- **Pipeline native** — JSONL output feeds directly into `hash`, `fingerprint`, and `lock` via Unix pipes.

---

## Quick Example

```bash
$ vacuum /data/dec
```

```jsonl
{"version":"vacuum.v0","path":"/data/dec/model.xlsx","relative_path":"model.xlsx","root":"/data/dec","size":2481920,"mtime":"2025-12-31T12:00:00.000Z","extension":".xlsx","mime_guess":"application/vnd.openxmlformats-officedocument.spreadsheetml.sheet","tool_versions":{"vacuum":"0.1.0"}}
{"version":"vacuum.v0","path":"/data/dec/tape.csv","relative_path":"tape.csv","root":"/data/dec","size":847201,"mtime":"2025-12-15T08:30:00.000Z","extension":".csv","mime_guess":"text/csv","tool_versions":{"vacuum":"0.1.0"}}
```

Two files inventoried — sorted, typed, timestamped, ready for `hash`.

```bash
# Filter to specific extensions:
$ vacuum /data/dec --include "*.csv" --include "*.xlsx"

# Exclude scratch files:
$ vacuum /data --exclude "*.tmp" --exclude ".DS_Store"

# Multiple roots, single manifest:
$ vacuum /data/q3 /data/q4 > q3q4-manifest.jsonl

# Full pipeline into lockfile:
$ vacuum /data/dec | hash | fingerprint --fp csv.v0 \
    | lock --dataset-id "dec" > dec.lock.json
```

---

## Where vacuum Fits

`vacuum` is the **first tool** in the stream pipeline — it discovers what exists.

```
vacuum  →  hash  →  fingerprint  →  lock  →  pack
(scan)    (hash)    (template)     (pin)    (seal)
```

Each tool reads JSONL from stdin and emits enriched JSONL to stdout. `vacuum` starts the chain by walking directories and producing the initial manifest.

---

## What vacuum Is Not

`vacuum` does not replace downstream tools.

| If you need... | Use |
|----------------|-----|
| Compute SHA256/BLAKE3 hashes | [`hash`](https://github.com/cmdrvl/hash) |
| Match files against template definitions | [`fingerprint`](https://github.com/cmdrvl/fingerprint) |
| Pin artifacts into a self-hashed lockfile | [`lock`](https://github.com/cmdrvl/lock) |
| Check structural comparability of CSVs | [`shape`](https://github.com/cmdrvl/shape) |
| Explain numeric changes between CSVs | [`rvl`](https://github.com/cmdrvl/rvl) |
| Bundle into immutable evidence packs | [`pack`](https://github.com/cmdrvl/pack) |

`vacuum` only answers: **what files exist, how big are they, and when were they last modified?**

---

## The Two Outcomes

`vacuum` emits exactly one domain outcome. Note: there is **no exit code 1** — either the scan completes or it refuses.

### 1. SCAN_COMPLETE (exit `0`)

All roots were enumerated. Individual file-level failures (e.g., permission denied on a single file) are recorded as `_skipped` records in the output stream — they don't prevent a successful scan.

```bash
$ vacuum /data/dec
# exit 0 — all files inventoried (some may be _skipped)
```

### 2. REFUSAL (exit `2`)

Cannot begin scanning. The root directory doesn't exist, isn't readable, or a filesystem error prevents the scan from starting.

```json
{
  "code": "E_ROOT_NOT_FOUND",
  "message": "Root path does not exist",
  "detail": { "root": "/data/nonexistent/" },
  "next_command": null
}
```

Refusals always include the error code and detail.

---

## How vacuum Compares

| Capability | vacuum | `find` / `ls` | Custom script | `tree --json` |
|------------|--------|---------------|---------------|---------------|
| Deterministic sorted output | Yes | No | Depends | No |
| Structured JSONL records | Yes | No | You write it | Partial |
| Skipped file tracking | Yes (with warnings) | Silent | You write it | No |
| Multi-root interleaved output | Yes | No | You write it | No |
| Include/exclude glob filters | Yes | `find` only | You write it | No |
| MIME type guessing | Yes | No | You write it | No |
| Pipeline integration (hash/lock) | Yes | No | No | No |
| Audit trail (witness ledger) | Yes | No | No | No |

**When to use vacuum:**
- Start of a data pipeline — discover what artifacts exist before hashing and locking
- Audit and compliance — produce a reproducible inventory of a directory
- CI automation — machine-readable manifests that feed into downstream tools

**When vacuum might not be ideal:**
- You need recursive file search with complex predicates — use `find`
- You need file content analysis — use downstream tools (`hash`, `fingerprint`, `shape`)
- You need real-time filesystem watching — vacuum is a point-in-time snapshot

---

## Installation

### Homebrew (Recommended)

```bash
brew install cmdrvl/tap/vacuum
```

### Shell Script

```bash
curl -fsSL https://raw.githubusercontent.com/cmdrvl/vacuum/main/scripts/install.sh | bash
```

### From Source

```bash
cargo build --release
./target/release/vacuum --help
```

---

## CLI Reference

```bash
vacuum <ROOT>... [OPTIONS]
vacuum witness <query|last|count> [OPTIONS]
```

### Arguments

- `<ROOT>...`: One or more directories to scan. At least one required.

### Options

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--include <GLOB>` | string | all files | Include pattern (repeatable) |
| `--exclude <GLOB>` | string | none | Exclude pattern (repeatable) |
| `--no-follow` | flag | `false` | Do not follow symlinks |
| `--no-witness` | flag | `false` | Suppress witness ledger recording |
| `--describe` | flag | `false` | Print compiled `operator.json` to stdout, exit `0` |
| `--schema` | flag | `false` | Print JSONL record JSON schema, exit `0` |
| `--progress` | flag | `false` | Emit structured progress JSONL to stderr |
| `--version` | flag | `false` | Print `vacuum <semver>` to stdout, exit `0` |

### Exit Codes

| Code | Meaning |
|------|---------|
| `0` | SCAN_COMPLETE (all roots enumerated) |
| `2` | REFUSAL or CLI error |

### Streams

- `stdout`: JSONL manifest records (one per file)
- `stderr`: progress diagnostics (with `--progress`) or warnings

---

## Output Record

Every discovered file produces one `vacuum.v0` record:

```json
{
  "version": "vacuum.v0",
  "path": "/data/dec/tape.csv",
  "relative_path": "tape.csv",
  "root": "/data/dec",
  "size": 847201,
  "mtime": "2025-12-15T08:30:00.000Z",
  "extension": ".csv",
  "mime_guess": "text/csv",
  "tool_versions": { "vacuum": "0.1.0" }
}
```

| Field | Type | Nullable | Description |
|-------|------|----------|-------------|
| `version` | string | no | Always `"vacuum.v0"` |
| `path` | string | no | Absolute path (OS-native separators) |
| `relative_path` | string | no | Path relative to root (forward slashes) |
| `root` | string | no | Absolute path of scan root |
| `size` | u64 | no | File size in bytes |
| `mtime` | string | no | ISO 8601 UTC with millisecond precision |
| `extension` | string | yes | File extension including dot (null if none) |
| `mime_guess` | string | yes | MIME type from extension lookup (null if unknown) |
| `tool_versions` | object | no | `{ "vacuum": "<semver>" }` |

### Skipped Records

Files that can't be stat'd (permission denied, broken symlinks) produce a skipped record:

```json
{
  "version": "vacuum.v0",
  "path": "/data/dec/protected.xlsx",
  "relative_path": "protected.xlsx",
  "root": "/data/dec",
  "size": null,
  "mtime": null,
  "extension": ".xlsx",
  "mime_guess": "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
  "_skipped": true,
  "_warnings": [
    { "tool": "vacuum", "code": "E_FILE_PERMISSION", "message": "Cannot read file metadata", "detail": { "error": "Permission denied" } }
  ],
  "tool_versions": { "vacuum": "0.1.0" }
}
```

Skipped records flow downstream — `hash` passes them through, `lock` collects them in the `skipped` array.

---

## Refusal Codes

| Code | Trigger | Next Step |
|------|---------|-----------|
| `E_ROOT_NOT_FOUND` | Root path doesn't exist | Check path spelling and that directory exists |
| `E_ROOT_PERMISSION` | Can't read root directory | Check directory permissions |
| `E_IO` | Filesystem error preventing scan start | Check disk/mount health |

Multiple roots: fail-fast on the first failing root.

---

## Troubleshooting

### "E_ROOT_NOT_FOUND" — directory doesn't exist

Check that the path is correct and the directory exists:

```bash
ls -la /data/dec/  # verify directory exists
vacuum /data/dec
```

### "E_ROOT_PERMISSION" — can't read directory

You don't have permission to read the root directory:

```bash
ls -la /data/  # check permissions on parent
# Fix: adjust permissions or run with appropriate access
```

### Skipped files in output

Individual files may be skipped due to permission issues or broken symlinks. These still produce records with `_skipped: true` — they don't prevent the scan from completing (exit 0). Check the `_warnings` array for details:

```bash
vacuum /data/dec | jq 'select(._skipped == true) | .path'
```

### Empty output — no files found

The directory exists but contains no files matching your include/exclude patterns:

```bash
# Check what's actually in the directory:
ls -la /data/dec/

# Check your patterns:
vacuum /data/dec --include "*.csv"  # too narrow?
```

### Include/exclude patterns not matching

Patterns are matched against `relative_path` (forward-slash normalized), not the absolute path:

```bash
# Wrong — matching against absolute path:
vacuum /data --include "/data/*.csv"

# Right — matching against relative path:
vacuum /data --include "*.csv"
vacuum /data --include "subdir/*.csv"
```

---

## Limitations

| Limitation | Detail |
|------------|--------|
| **In-memory collection** | All records collected before emission (deterministic ordering requires it) |
| **Extension-based MIME** | MIME guessing uses file extension, not content sniffing — unknown extensions → `null` |
| **No content hashing** | vacuum doesn't read file contents — use `hash` for that |
| **No recursive exclude** | `--exclude` patterns match against relative paths, not directory tree structure |
| **Point-in-time snapshot** | No file watching — re-run vacuum to detect changes |
| **No exit code 1** | Per-file failures are `_skipped` records, not partial outcomes (unlike `hash`/`lock`) |

---

## FAQ

### Why a separate tool instead of using `find`?

`find` produces unstructured text that requires parsing. vacuum produces deterministic JSONL with rich metadata (size, mtime, MIME type, extension) that pipes directly into the rest of the pipeline. Same directory always produces identical output.

### Why is there no exit code 1?

vacuum's job is enumeration, not transformation. Either the scan starts (exit 0) or it can't (exit 2). Per-file issues like permission denied are recorded as `_skipped` records in the output stream — they don't prevent the scan from completing.

### Why does vacuum collect all records before emitting?

Deterministic ordering. Records are sorted by `(relative_path, root)` byte-order, which requires seeing all files before emitting any. This trades latency for reproducibility.

### How does multi-root scanning work?

Records from all roots are interleaved by `relative_path`, with `root` as tiebreaker. This means files with the same relative path from different roots appear adjacent in the output.

### Can I scan multiple directories?

Yes — pass multiple roots:

```bash
vacuum /data/q3 /data/q4 /data/q1
```

All files from all roots appear in a single sorted manifest.

### How are symlinks handled?

By default, vacuum follows symlinks and resolves targets to canonical paths. Use `--no-follow` to skip symlinks entirely.

### What MIME types does vacuum recognize?

Extension-based lookup from a built-in table: `.csv`, `.tsv`, `.txt`, `.json`, `.jsonl`, `.xml`, `.pdf`, `.xlsx`, `.xls`, `.parquet`, `.zip`, `.gz`, `.yaml`/`.yml`, and others. Unknown extensions produce `null`.

---

## Agent / CI Integration

### Self-describing contract

```bash
$ vacuum --describe | jq '.exit_codes'
{
  "0": { "meaning": "SCAN_COMPLETE" },
  "2": { "meaning": "REFUSAL" }
}

$ vacuum --describe | jq '.pipeline'
{
  "upstream": [],
  "downstream": ["hash", "fingerprint", "lock", "pack"]
}
```

### Agent workflow

```bash
# 1. Scan directory
vacuum /data/dec > manifest.jsonl

case $? in
  0) echo "scan complete"
     wc -l manifest.jsonl ;;
  2) echo "refusal"
     cat manifest.jsonl | jq '.code'
     exit 1 ;;
esac

# 2. Count skipped files
skipped=$(jq -s '[.[] | select(._skipped == true)] | length' manifest.jsonl)
echo "skipped: $skipped"

# 3. Pipe into rest of pipeline
cat manifest.jsonl | hash | lock --dataset-id "dec" > dec.lock.json
```

### What makes this agent-friendly

- **Exit codes** — `0`/`2` map to success/error branching (no ambiguous exit 1)
- **Structured JSONL only** — stdout is always machine-readable
- **`--describe`** — prints `operator.json` so an agent discovers the tool without reading docs
- **`--schema`** — prints the record JSON schema for programmatic validation
- **Skipped records inline** — agents can filter `_skipped` records without separate error streams

---

<details>
<summary><strong>Witness Subcommands</strong></summary>

`vacuum` records every scan to an ambient witness ledger. You can query this ledger:

```bash
# Query by date range or outcome
vacuum witness query --tool vacuum --since 2026-01-01 --outcome SCAN_COMPLETE --json

# Get the most recent scan
vacuum witness last --json

# Count scans matching a filter
vacuum witness count --since 2026-02-01
```

### Subcommand Reference

```bash
vacuum witness query [--tool <name>] [--since <iso8601>] [--until <iso8601>] \
  [--outcome <SCAN_COMPLETE|REFUSAL>] [--input-hash <substring>] \
  [--limit <n>] [--json]

vacuum witness last [--json]

vacuum witness count [--tool <name>] [--since <iso8601>] [--until <iso8601>] \
  [--outcome <SCAN_COMPLETE|REFUSAL>] [--input-hash <substring>] [--json]
```

### Exit Codes (witness subcommands)

| Code | Meaning |
|------|---------|
| `0` | One or more matching records returned |
| `1` | No matches (or empty ledger for `last`) |
| `2` | CLI parse error or witness internal error |

### Ledger Location

- Default: `~/.epistemic/witness.jsonl`
- Override: set `EPISTEMIC_WITNESS` environment variable
- Malformed ledger lines are skipped; valid lines continue to be processed.

</details>

---

## Spec and Development

The full specification is [`docs/PLAN.md`](./docs/PLAN.md). This README covers intended v0 behavior; the spec adds implementation details, edge-case definitions, and testing requirements.

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```
