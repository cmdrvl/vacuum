# Fixture Corpus

This directory provides deterministic fixture inputs for `vacuum` scanner tests.

## `simple/`
- Flat directory with common formats (`.csv`, `.txt`, `.json`).
- Use for baseline enumeration, extension extraction, and MIME guess assertions.

## `nested/`
- Multi-level tree with files at different depths.
- Use for relative path normalization and recursive ordering checks.

## `empty_dir/`
- Logical empty-directory fixture.
- `.gitkeep` exists only to preserve the directory in git; scanner tests should treat this fixture as empty by removing/copying without `.gitkeep` when asserting zero records.

## `symlinks/`
- Contains a file symlink (`file_link.txt`), directory symlink (`dir_link`), and broken symlink (`broken_link`).
- Use for follow/no-follow behavior and broken-link skip handling.

## `mixed/`
- Mixed visibility and naming patterns (`.hidden.json`, `no_extension`, nested files).
- Use for include/exclude matching and extension/null MIME edge coverage.

## Stability
- Fixture files are ASCII and intentionally short.
- File contents should remain stable to keep deterministic size/order assertions reliable.
