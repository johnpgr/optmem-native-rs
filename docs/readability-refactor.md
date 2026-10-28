# Readability refactor validation

Completed locally on 2026-10-21, Apple Silicon macOS, Rust 1.98.1, Python 3.9.6.

## Changes

The previous 922-line `main.rs` is now a 25-line entry point. Process I/O lives
in `cli`; per-invocation state in `app`; command handlers in `commands/`; and
configuration, compression prompts, output buffering, pagination, and shared
validation each have a defined owner. Storage is split into record codec, raw
log, summary tree, locks, physical files, and pending-work accounting.

The source now uses named configuration fields and typed setting identities,
named wake/recall state, and named dated-memory append input. The custom output
macro was replaced with an output buffer that preserves stdout preceding an
error. Wake and nap use focused helpers and guard clauses. Module/item contracts
and logical spacing explain the storage, snapshot, byte-budget, and lock rules.

The largest source file is now 262 lines (`paths.rs`). There are 30 Rust source
files, grouped by responsibility. The README maps those responsibilities, and
AGENTS.md records the user's readability standards for future changes.

Dependencies, release settings, the frozen Python reference, the nine commands,
fixed-record formats, and documented compatibility differences are unchanged.
The record scanner remains per-record with borrowed canonical text and the
existing eight-byte padding trim. No installation was performed.

## Correctness checks

- Formatting and strict Clippy with all targets pass.
- All 33 tests pass in debug and release: 12 unit/cover tests, 2 concurrency
  tests, 6 deliberate-difference tests, 11 differential CLI tests, and 2 native
  regression tests. Python was available, so oracle checks actually ran.
- Private-item documentation builds with warnings denied.
- Linux x86-64 and Windows x86-64 MSVC all-target compile checks pass. These
  checks do not establish native runtime behavior on either OS.
- A separate comparison against the saved pre-refactor release binary matched
  stdout, stderr, status, and store bytes across 120 commands, normalizing only
  executable/store paths. It covers initialization, import, full compression,
  configuration/reset, pagination with intervening writes, tree invalidation,
  recall caps, and oversized numeric arguments.

The added regression tests exercise public behavior: oversized block IDs must
not alter saved summaries, and greedy regexes must respect record boundaries
and the output cap. Existing formatter failures in a test helper were corrected.

## Performance comparison

Both binaries used the same 1M-record synthetic store with varied payload lengths
and sparse Unicode/literal matches. Each expression checked output equality,
warmed both binaries, and used seven whole-process runs in seeded random order.
File caches were warm. Peak RSS came from a separate `/usr/bin/time -l` run.

| Expression | Before | After |
|---|---:|---:|
| `elixir` | 76.2 ms | 80.2 ms |
| Absent literal | 76.2 ms | 79.9 ms |
| `.` | 84.5 ms | 88.5 ms |
| `coração` | 76.4 ms | 79.8 ms |
| `(elixir\|coração\|otp\s+29)` | 114.2 ms | 118.9 ms |
| `elixir.*$` | 107.7 ms | 109.7 ms |
| `^#\d+` | 85.5 ms | 88.7 ms |
| `(?s:.*)` | 88.8 ms | 92.2 ms |

The refactor has a small measured recall cost, roughly 2–5% in this run.
A prior comparison showed a similar difference. Inlining hints on the extracted
record helpers did not remove it. These results should not be described as a
zero-regression performance result, nor extrapolated to cold-cache or other OSes.

Recall peak RSS was approximately 3.7–4.1 MiB after versus 3.8–4.3 MiB before.
The executable decreased from 1,411,968 to 1,395,392 bytes. RSS measurements are
single samples per expression and do not establish a precise memory improvement.
Bulk import still stages input and a padded batch; its memory tradeoff was not
changed by this refactor.

## Local artifacts

`target/refactor-review/` contains the original source snapshot and release
binary, `checks.json`, `compare_before_after.py`, `before-after.json`, and the
initial comparison data. These are local build artifacts and are removed by
`cargo clean`. The behavioral fixture and benchmark use only disposable stores.
