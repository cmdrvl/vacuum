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

## `deeply_nested/`
- Deep path hierarchy ending in `deep.txt`.
- Use for recursion depth and relative-path normalization checks.

## `symlink_chains/`
- Symlink chain `link3 -> link2 -> link1 -> target.txt`.
- Use for chained symlink resolution and no-follow behavior tests.

## `permission_denied/`
- Contains `locked.txt` plus guidance for runtime permission mutation.
- Use for tests that `chmod 000` at runtime to assert `_skipped` handling.

## `empty_dirs/`
- Multiple nested empty directories preserved with `.gitkeep`.
- Use for empty-directory traversal edge cases.

## `unicode_space/`
- Filenames with spaces and Unicode (`hello world.txt`, `café.csv`).
- Use for path encoding and platform filename handling checks.

## `large_file_counts/`
- 30 deterministic small files (`file_1.txt` ... `file_30.txt`).
- Use for larger directory cardinality and ordering coverage.

## `mixed_binary_text/`
- Contains both binary (`sample.bin`) and text (`sample.txt`) files.
- Use for metadata-only scanning behavior independent of content type.

## `hidden_dotfiles/`
- Hidden files (`.env`, `.config.yml`).
- Use for hidden-entry inclusion/exclusion behavior checks.

## `zero_byte/`
- Zero-byte file (`empty.dat`).
- Use for size=`0` metadata assertions.

## `no_extension_cases/`
- Files without extensions (`README`, `data`).
- Use for extension=`null` and MIME=`null` assertions.

## Stability
- Fixture files are intentionally short; `unicode_space/` intentionally includes non-ASCII names.
- File contents should remain stable to keep deterministic size/order assertions reliable.
