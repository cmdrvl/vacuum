# vacuum — Artifact Inventory

## One-line promise

**Enumerate every artifact in scope and produce a stable, deterministic manifest.**

Does not hash, parse, extract, or interpret. Just names what's there.

Second promise: **Know what you have before you touch it.**

---

## Problem (clearly understood)

Before you can hash, fingerprint, compare, or lock a set of files, you need to know what files exist. Today this means:

- `find` / `ls -R` with ad-hoc scripts
- Inconsistent metadata (some tools report mtime, some don't)
- No stable ordering (output order depends on filesystem, OS, and inode allocation)
- No structured output (grep-and-awk pipelines to extract paths)
- No evidence that the scan itself was complete (silent permission errors)

`vacuum` replaces that with **one trusted inventory command** that produces a deterministic JSONL manifest.

---

## Non-goals (explicit)

`vacuum` is NOT:

- A hasher (that's `hash`)
- A template recognizer (that's `fingerprint`)
- A lockfile generator (that's `lock`)
- A diff tool (that's `compare` / `rvl`)
- A file parser (it reads metadata, never file content beyond magic bytes for MIME)

It does not tell you *what's inside* a file.
It tells you *that the file exists, where it is, how big it is, and when it was last modified*.

---

## Relationship to the pipeline

`vacuum` is the first tool in the stream pipeline. Its output feeds `hash`, which feeds `fingerprint`, which feeds `lock`:

```bash
vacuum /data/2025-12/ | hash | fingerprint --fp argus-model.v1 | lock --dataset-id "dec" > dec.lock.json
```

vacuum can also be used standalone to inventory a delivery before any processing:

```bash
vacuum /data/2025-12/ > manifest.jsonl
```

All downstream stream tools expect vacuum's JSONL record schema as their baseline input. vacuum sets the record shape that flows through the entire pipeline.

---

## CLI (v0)

```bash
vacuum <ROOT>... [OPTIONS]
vacuum witness <query|last|count> [OPTIONS]
```

### Arguments

- `<ROOT>...`: One or more root directories to scan. At least one required.

### Flags (v0.1 — core)

- `--include <GLOB>`: Include pattern (repeatable; default: all files). Standard glob syntax (`*.pdf`, `*.xlsx`, `**/*.csv`).
- `--exclude <GLOB>`: Exclude pattern (repeatable). Applied after include. Matches against `relative_path`.
- `--no-follow`: Do not follow symlinks (default: follow symlinks).
- `--no-witness`: Suppress witness ledger recording for this run.
- `--describe`: Print the compiled-in `operator.json` to stdout and exit 0. Checked before root arguments are validated, so `vacuum --describe` works with no positional args.
- `--schema`: Print the JSON Schema for the JSONL record to stdout and exit 0.
- `--progress`: Emit structured progress JSONL to stderr (see Progress reporting).
- `--version`: Print `vacuum <semver>` to stdout and exit 0.

### Exit codes

- `0`: Scan completed (all roots enumerated successfully)
- `2`: Refusal / CLI error

There is no exit code `1` for vacuum. vacuum either scans successfully or refuses. There is no partial-success state at the scan level — per-file issues (permission denied on individual files) are recorded as `_skipped` records in the output stream and cause exit `0` (the scan completed; the failure is recorded as evidence).

> **Design note:** exit `1` is reserved for domain-negative outcomes in the spine convention (e.g., rvl's REAL_CHANGE, shape's INCOMPATIBLE). vacuum has no domain-negative outcome — a successful scan is always positive regardless of what it finds.

### Streams

- JSONL records to stdout (always structured; no human mode for the manifest itself).
- Progress (when `--progress`): structured JSONL to stderr.
- Warnings (without `--progress`): unstructured one-per-line to stderr.

### Witness ledger (epistemic spine parity)

`vacuum` follows the same ambient witness protocol as `rvl` and `shape`:

- Default behavior: every scan run appends exactly one `witness.v0` record.
- Opt-out: `--no-witness`.
- Ledger path resolution:
  1. `EPISTEMIC_WITNESS` env var, if set
  2. `~/.epistemic/witness.jsonl` otherwise
- Witness failures never change the domain exit code.

Witness query subcommands:

```bash
vacuum witness query [--tool <name>] [--since <iso8601>] [--until <iso8601>] \
  [--outcome <SCAN_COMPLETE|REFUSAL>] [--input-hash <substring>] \
  [--limit <n>] [--json]

vacuum witness last [--json]
vacuum witness count [--tool <name>] [--since <iso8601>] [--until <iso8601>] \
  [--outcome <SCAN_COMPLETE|REFUSAL>] [--input-hash <substring>] [--json]
```

---

## Outcomes (exactly one)

### 1. SCAN_COMPLETE

All roots were enumerated. The manifest is on stdout. Individual file-level failures (permission denied, broken symlinks) are recorded as `_skipped` records in the stream — they do not prevent the scan from completing.

### 2. REFUSAL

When vacuum cannot begin scanning (root doesn't exist, root can't be read, CLI error). Always includes a concrete next step.

No other outcomes.

---

## Output Record Schema (`vacuum.v0`)

Each record is a single JSON object on one line (JSONL). One record per discovered artifact.

```json
{
  "version": "vacuum.v0",
  "path": "/data/2025-12/tape.csv",
  "relative_path": "tape.csv",
  "root": "/data/2025-12",
  "size": 48291,
  "mtime": "2025-12-31T12:00:00.000Z",
  "extension": ".csv",
  "mime_guess": "text/csv",
  "tool_versions": { "vacuum": "0.1.0" }
}
```

### Field definitions

| Field | Type | Nullable | Notes |
|-------|------|----------|-------|
| `version` | string | no | Always `"vacuum.v0"` |
| `path` | string | no | Absolute path (OS-native separators for filesystem access) |
| `relative_path` | string | no | Path relative to `root`, normalized to forward slashes |
| `root` | string | no | Absolute path of the scan root this file belongs to |
| `size` | u64 | no | File size in bytes (from filesystem metadata) |
| `mtime` | string | no | Last modified time, ISO 8601 UTC with millisecond precision |
| `extension` | string | yes | File extension including dot (e.g., `.csv`, `.xlsx`); `null` if no extension |
| `mime_guess` | string | yes | MIME type guessed from extension; `null` if unknown |
| `tool_versions` | object | no | `{ "vacuum": "<semver>" }` — accumulated by downstream tools |

### Skipped records

When vacuum encounters a file it cannot stat (permission denied, broken symlink, etc.), it emits a skipped record:

```json
{
  "version": "vacuum.v0",
  "path": "/data/2025-12/protected.xlsx",
  "relative_path": "protected.xlsx",
  "root": "/data/2025-12",
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

For skipped records:
- `size` and `mtime` are `null` (metadata couldn't be read)
- `path`, `relative_path`, `root`, `extension`, `mime_guess` are populated from the directory entry (which was readable even if the file isn't)
- `_skipped` is `true`
- `_warnings` contains one or more warning objects

### Ordering

Records are emitted sorted by `relative_path` (lexicographic, byte-order). This requires vacuum to collect all entries during the scan, then sort and emit. vacuum does NOT emit records as they are discovered — deterministic ordering requires seeing all paths first.

When multiple roots are provided, records from all roots are interleaved by `relative_path`. If the same `relative_path` appears under different roots, records are further sorted by `root` (lexicographic, byte-order).

### Path normalization

- `relative_path` always uses forward slashes (`/`), regardless of OS.
- `path` and `root` use OS-native separators (for filesystem access).
- Symlink targets are resolved to their canonical path when `--no-follow` is not set; the `path` field shows the resolved path.

### MIME guessing

MIME type is guessed from the file extension using a built-in lookup table. This is a cheap heuristic, not a content-based detection. The `infer` crate (magic-bytes detection) is a candidate for v0.1 enhancement but not required for v0.

Built-in extension-to-MIME mappings (minimum set):

| Extension | MIME |
|-----------|------|
| `.csv` | `text/csv` |
| `.tsv` | `text/tab-separated-values` |
| `.txt` | `text/plain` |
| `.json` | `application/json` |
| `.jsonl` | `application/x-jsonlines` |
| `.xml` | `application/xml` |
| `.pdf` | `application/pdf` |
| `.xlsx` | `application/vnd.openxmlformats-officedocument.spreadsheetml.sheet` |
| `.xls` | `application/vnd.ms-excel` |
| `.parquet` | `application/vnd.apache.parquet` |
| `.zip` | `application/zip` |
| `.gz` | `application/gzip` |
| `.yaml` / `.yml` | `application/x-yaml` |

Anything not in the table → `null`.

### `--include` / `--exclude` patterns

Glob patterns are matched against `relative_path` (forward-slash normalized). Standard glob syntax:

- `*` matches any sequence of non-separator characters
- `**` matches any sequence of characters including separators
- `?` matches a single non-separator character
- `[abc]` character class

Evaluation order:
1. If `--include` patterns are provided, a file must match at least one include pattern to be considered.
2. If `--exclude` patterns are provided, a file matching any exclude pattern is dropped (even if it matched an include).
3. If no `--include` is specified, all files are included by default.

Directories are always traversed regardless of include/exclude — patterns only filter leaf files.

---

## Refusal Codes

| Code | Trigger | Next step |
|------|---------|-----------|
| `E_ROOT_NOT_FOUND` | Root path doesn't exist | Check path spelling and that the directory exists |
| `E_ROOT_PERMISSION` | Can't read root directory (not individual files) | Check directory permissions |
| `E_IO` | Filesystem error preventing scan start | Check disk/mount |

> **Note:** Per-file errors (individual files that can't be stat'd) are NOT refusals. They are recorded as `_skipped` records in the output stream. Refusals are reserved for root-level failures that prevent the scan from starting.

Refusal envelope (same as all spine tools):

```json
{
  "code": "E_ROOT_NOT_FOUND",
  "message": "Root path does not exist",
  "detail": { "root": "/data/nonexistent/" },
  "next_command": null
}
```

### Refusal detail schemas

```
E_ROOT_NOT_FOUND:
  { "root": "/data/nonexistent/" }

E_ROOT_PERMISSION:
  { "root": "/data/restricted/", "error": "Permission denied" }

E_IO:
  { "root": "/data/broken-mount/", "error": "Input/output error" }
```

### Multiple roots: fail-fast behavior

If any root fails validation (doesn't exist, can't be read), vacuum refuses immediately and reports the first failing root. It does not attempt to scan the remaining roots.

---

## Progress Reporting (`--progress`)

When `--progress` is provided, vacuum emits structured JSONL to stderr:

```jsonl
{"type": "progress", "tool": "vacuum", "processed": 5000, "total": null, "elapsed_ms": 1200}
{"type": "progress", "tool": "vacuum", "processed": 10000, "total": null, "elapsed_ms": 2350}
{"type": "warning", "tool": "vacuum", "path": "/data/corrupt.pdf", "message": "skipped: permission denied"}
```

- `total` is `null` because recursive directory scanning discovers files as it goes — the total is not known upfront.
- Progress records are emitted at regular intervals (every 1000 files or every 500ms, whichever comes first).
- Warning records are emitted immediately when a file is skipped.
- Without `--progress`, stderr only has unstructured warning lines (one per line).

---

## Implementation Notes

### Execution flow

```
 1. Parse CLI args (clap)           → exit 2 on bad args; --version handled here by clap
 2. If --describe: print operator.json to stdout, exit 0
 3. If --schema: print JSON Schema to stdout, exit 0
 4. Validate all roots exist and are readable directories
    → E_ROOT_NOT_FOUND / E_ROOT_PERMISSION on first failure (STOP)
 5. Walk all roots, collecting file entries
    → Per-file failures become _skipped records (NOT refusals)
    → Apply --include / --exclude filters
 6. Sort collected entries by (relative_path, root)
 7. Emit sorted JSONL to stdout
 8. Append witness record (if not --no-witness)
 9. Exit 0
```

### Core data structures

```rust
// === CLI ===

#[derive(Parser)]
pub struct Args {
    /// Root directories to scan
    #[arg(required = true)]
    pub roots: Vec<PathBuf>,

    /// Include glob pattern (repeatable)
    #[arg(long, action = clap::ArgAction::Append)]
    pub include: Vec<String>,

    /// Exclude glob pattern (repeatable)
    #[arg(long, action = clap::ArgAction::Append)]
    pub exclude: Vec<String>,

    /// Do not follow symlinks
    #[arg(long)]
    pub no_follow: bool,

    /// Suppress witness ledger recording
    #[arg(long)]
    pub no_witness: bool,

    /// Emit progress to stderr
    #[arg(long)]
    pub progress: bool,

    /// Print operator.json and exit
    #[arg(long)]
    pub describe: bool,

    /// Print JSON Schema and exit
    #[arg(long)]
    pub schema: bool,
}

// === Output record ===

#[derive(Serialize)]
pub struct VacuumRecord {
    pub version: &'static str,           // "vacuum.v0"
    pub path: String,                     // absolute, OS-native
    pub relative_path: String,            // forward-slash normalized
    pub root: String,                     // absolute, OS-native
    pub size: Option<u64>,                // null when _skipped
    pub mtime: Option<String>,            // ISO 8601 UTC; null when _skipped
    pub extension: Option<String>,        // including dot; null if none
    pub mime_guess: Option<String>,       // from extension; null if unknown
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _skipped: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _warnings: Option<Vec<Warning>>,
    pub tool_versions: BTreeMap<String, String>,
}

#[derive(Serialize)]
pub struct Warning {
    pub tool: String,
    pub code: String,
    pub message: String,
    pub detail: serde_json::Value,
}

// === Refusal ===

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefusalCode {
    RootNotFound,
    RootPermission,
    Io,
}

impl RefusalCode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::RootNotFound => "E_ROOT_NOT_FOUND",
            Self::RootPermission => "E_ROOT_PERMISSION",
            Self::Io => "E_IO",
        }
    }

    pub fn reason(&self) -> &'static str {
        match self {
            Self::RootNotFound => "Root path does not exist",
            Self::RootPermission => "Cannot read root directory",
            Self::Io => "Filesystem error during scan",
        }
    }
}

pub struct RefusalPayload {
    pub code: RefusalCode,
    pub message: &'static str,
    pub detail: serde_json::Value,
    pub next_command: Option<String>,
}
```

### Directory walking

Use the `walkdir` crate for recursive directory traversal:
- Follow symlinks by default; `--no-follow` disables
- Detect and break symlink cycles (walkdir handles this)
- On per-entry errors (permission denied, broken symlink): emit a `_skipped` record with the path and warning, continue walking

### Glob matching

Use the `globset` crate for `--include` / `--exclude` pattern matching:
- Compile all include patterns into a single `GlobSet`
- Compile all exclude patterns into a single `GlobSet`
- Match against `relative_path` (forward-slash normalized)

### Sorting strategy

All entries are collected into a `Vec<VacuumRecord>` during the walk, then sorted by `(relative_path, root)` before emission. This is required for deterministic output.

For very large scans (millions of files), this means the full manifest is held in memory before any output. This is acceptable for v0 — the manifest records are small (~200-300 bytes each), so 1M files ≈ 200-300 MB of records.

### Module structure

```
src/
├── cli/
│   ├── args.rs          # clap derive Args struct
│   ├── exit.rs          # Exit code mapping
│   └── mod.rs
├── walk/
│   ├── walker.rs        # Directory walking with walkdir
│   ├── filter.rs        # Include/exclude glob filtering
│   └── mod.rs
├── record/
│   ├── builder.rs       # VacuumRecord construction from DirEntry
│   ├── mime.rs          # Extension-to-MIME lookup table
│   ├── path.rs          # Path normalization (forward slashes)
│   └── mod.rs
├── output/
│   ├── jsonl.rs         # JSONL serialization to stdout
│   └── mod.rs
├── progress/
│   ├── reporter.rs      # Structured progress to stderr
│   └── mod.rs
├── refusal/
│   ├── codes.rs         # RefusalCode enum
│   ├── payload.rs       # RefusalPayload construction
│   └── mod.rs
├── witness/
│   ├── record.rs        # Witness record construction
│   ├── ledger.rs        # Append to witness ledger
│   ├── query.rs         # Witness query subcommands
│   └── mod.rs
├── lib.rs               # pub fn run() → Result<u8, Box<dyn Error>>
└── main.rs              # Minimal: calls vacuum::run(), maps to ExitCode
```

### `main.rs` (≤15 lines)

```rust
#![forbid(unsafe_code)]

fn main() -> std::process::ExitCode {
    let code = vacuum::run();
    std::process::ExitCode::from(code)
}
```

---

## Witness Record

vacuum's witness record follows the standard `witness.v0` schema:

```json
{
  "id": "blake3:...",
  "tool": "vacuum",
  "version": "0.1.0",
  "binary_hash": "blake3:...",
  "inputs": [
    { "path": "/data/2025-12", "hash": null, "bytes": null }
  ],
  "params": { "roots": ["/data/2025-12"], "include": ["*.csv"], "exclude": [], "no_follow": false },
  "outcome": "SCAN_COMPLETE",
  "exit_code": 0,
  "output_hash": "blake3:...",
  "prev": "blake3:...",
  "ts": "2026-02-24T10:00:00Z"
}
```

For vacuum, `inputs[].hash` and `inputs[].bytes` are `null` because roots are directories, not hashable files. The `output_hash` is the BLAKE3 hash of the full JSONL output.

---

## Operator Manifest (`operator.json`)

```json
{
  "schema_version": "operator.v0",
  "name": "vacuum",
  "version": "0.1.0",
  "description": "Enumerates artifacts in scope, emitting a stable JSONL manifest of files and metadata",
  "repository": "https://github.com/cmdrvl/vacuum",
  "license": "MIT",

  "invocation": {
    "binary": "vacuum",
    "output_mode": "stream",
    "output_schema": "vacuum.v0",
    "json_flag": null
  },

  "arguments": [
    { "name": "roots", "type": "file_path[]", "required": true, "variadic": true, "description": "Root directories to scan" }
  ],

  "options": [
    { "name": "include", "flag": "--include", "type": "string", "repeatable": true, "description": "Include glob pattern" },
    { "name": "exclude", "flag": "--exclude", "type": "string", "repeatable": true, "description": "Exclude glob pattern" },
    { "name": "no_follow", "flag": "--no-follow", "type": "boolean", "description": "Do not follow symlinks" }
  ],

  "exit_codes": {
    "0": { "meaning": "SCAN_COMPLETE", "domain": "positive" },
    "2": { "meaning": "REFUSAL", "domain": "error" }
  },

  "refusals": [
    { "code": "E_ROOT_NOT_FOUND", "message": "Root path doesn't exist", "action": "escalate" },
    { "code": "E_ROOT_PERMISSION", "message": "Can't read root", "action": "escalate" },
    { "code": "E_IO", "message": "Filesystem error during scan", "action": "escalate" }
  ],

  "capabilities": {
    "formats": ["*"],
    "profile_aware": false,
    "streaming": true
  },

  "pipeline": {
    "upstream": [],
    "downstream": ["hash"]
  }
}
```

---

## Testing Requirements

### Fixtures

Provide test fixtures in `tests/fixtures/`:

- `simple/` — a directory with 3-5 files of different types (csv, xlsx, pdf, txt)
- `nested/` — multi-level directory structure
- `empty_dir/` — empty directory (produces no records)
- `symlinks/` — directory with symlinks (both file and directory symlinks)
- `mixed/` — directory with some readable and some unreadable files (for `_skipped` testing)

### Test categories

- **Basic scan tests:** single root, multiple roots, nested directories
- **Ordering tests:** output is sorted by `(relative_path, root)` deterministically
- **Include/exclude tests:** glob patterns filter correctly
- **Symlink tests:** follow by default, `--no-follow` skips, cycle detection
- **Skipped record tests:** permission-denied files produce `_skipped` records with `_warnings`
- **MIME guessing tests:** known extensions map correctly, unknown → null
- **Path normalization tests:** `relative_path` uses forward slashes on all platforms
- **Multiple roots tests:** records from multiple roots interleave correctly
- **Refusal tests:** each refusal code triggered correctly (nonexistent root, unreadable root)
- **Exit code tests:** 0 for successful scan, 2 for refusal
- **Idempotency tests:** same directory scanned twice produces identical JSONL output
- **Witness tests:** witness record is appended, `--no-witness` suppresses it, query subcommands work
- **`--describe` test:** prints valid operator.json
- **`--schema` test:** prints valid JSON Schema
- **`--progress` test:** structured JSONL on stderr when flag is set

### Golden file tests

- Scan `simple/` → compare output against a golden JSONL file
- Scan with `--include "*.csv"` → only CSV files in output
- Scan with `--exclude "*.tmp"` → tmp files excluded

---

## Scope: v0.1 (ship this)

### Must have

- `<ROOT>...` positional args (one or more)
- `--include <GLOB>` / `--exclude <GLOB>` (repeatable)
- `--no-follow` flag
- Deterministic JSONL output sorted by `(relative_path, root)`
- `_skipped` / `_warnings` for per-file failures
- `tool_versions` accumulator
- Extension-based MIME guessing
- Ambient witness recording + `--no-witness`
- `vacuum witness <query|last|count>` subcommands
- `--version` flag
- `operator.json` + `--describe`
- Exit codes 0/2
- Refusal system with `E_ROOT_NOT_FOUND`, `E_ROOT_PERMISSION`, `E_IO`

### Can defer

- `--schema` flag (JSON Schema output)
- `--progress` flag (structured progress)
- Magic-bytes MIME detection (via `infer` crate)
- Concurrent directory walking (single-threaded walkdir is fast enough for v0)
- Remote scanning backends (S3, SharePoint)
- `--max-files` / `--max-depth` guardrails

---

## Open Questions

*None currently blocking. Build it.*
