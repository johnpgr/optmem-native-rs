# optmem-native

This is a native Rust port of the [OptMem](https://github.com/VictorTaelin/OptMem)
CLI (`memo`). It has the same nine commands and arguments, the same agent-facing
text, the same exit statuses, and the same on-disk format. You can put it in
place of the Python script, on an existing store, with no migration.

Status: implemented and tested on macOS (Apple Silicon). The Linux and Windows
builds pass type checking and lint, but no tests have run on them yet; the CI
workflow does that. The refactored binary is **installed locally** at
`~/.optmem/memo` as of 2026-10-21. The Python CLI is backed up; see
[local installation and tests](docs/local-installation.md) for verification and rollback.

## Build

```sh
cargo build --release        # target/release/memo (memo.exe on Windows)
cargo test                   # unit, differential, difference and lock tests
```

Python is needed only for the tests and the benchmark, never at run time. The
tests use `python3`, or the interpreter in `$OPTMEM_PYTHON`, to run the frozen
reference at `reference/memo.py`. That file is a copy of the installed script
on 2026-10-21. Its SHA-256 is in `reference/memo.py.sha256`:
`3dc120d01be3115ef6267eab4103e7909fc830d6227b549f20991ba999ee9ffb`.
If Python is not available, the differential tests are skipped.

## What is kept exactly

- The commands `init`, `wake`, `note`, `nap`, `recall`, `zoom`, `forget`,
  `config` and `import`, with the same arguments, usage lines, prompts,
  errors and exit statuses.
- The store at `~/.optmem/memory`, or `$MEMORY_DIR` (`~` is expanded and an
  empty value means the default). Only `init` creates a store.
- The record formats: 320-byte `LOG.txt` records and 288-byte `TREE/<size>`
  records. Each record is UTF-8 text, padded with spaces, with one final
  newline.
- The wake tiling (`cover`, including its float bisection), pagination, the
  `wake <part> <T>` snapshot, nap ordering, the raw-summary threshold of 16
  and the config defaults. Validation and `config` file rewriting are the
  same.
- Byte-based limits. Python whitespace rules for `strip` (Unicode
  White_Space, plus U+001C–U+001F). Universal newlines when the tool reads
  `config` and import files. The local calendar date for notes.
- Locking. The tool appends under an exclusive lock and assigns IDs inside
  that lock. It repairs a torn tail under the lock, then flushes and calls
  `fsync`. On Unix, the lock is `flock(2)` on `.lock`, as with Python's
  `fcntl.flock`. On Windows, it is a `LockFileEx` range that covers the byte
  that Python's `msvcrt.locking` locks. Native and Python writers can share
  one store.
- The executable path in printed commands. Python prints `__file__` with
  symlinks kept and `$HOME` shown as `~`. The native tool rebuilds this path
  from `argv[0]`, and searches `PATH` for a bare name as the shell does. On
  Windows, it uses the module path.

The differential tests check all of this. They run the same command sequences
against the reference and the native tool, on separate disposable homes. Then
they compare stdout, stderr, the exit status and every store file byte for
byte. The sequences include seeded random ones. A separate test compares
`cover` with the reference for about 6,000 (T, budget) pairs, up to
T = 123,456,789.

## Source organization

The implementation separates process I/O, command behavior, and storage. Named
configuration fields distinguish line limits from UTF-8 byte budgets. Command
handlers use guard clauses and separate validation, work, and response sections.

| Location | Responsibility |
| --- | --- |
| `src/main.rs`, `src/cli.rs` | Entry point, OS arguments, dispatch, final stdout/stderr and status |
| `src/app.rs` | Per-invocation state, explicit store opening, loaded configuration |
| `src/commands/` | Focused handlers for initialization, wake, note, nap, recall, config, import, and tree navigation |
| `src/config.rs` | Named limits, typed setting identities, override/reset semantics, stable config format |
| `src/compression.rs`, `src/cover.rs` | Compression planning/prompts and exact snapshot coverage math |
| `src/store/` | Record codec, aligned log scanner, tree levels, physical I/O, lock ownership, pending-work counts |
| `src/output.rs`, `src/pagination.rs` | Buffered responses, platform newlines, byte/line-limited wake pages |
| `src/validation.rs`, `src/text.rs`, `src/pystr.rs` | Shared validation, UTF-8 reads, Python text compatibility |
| `src/paths.rs`, `src/today.rs`, `src/fail.rs` | Platform paths/date and structured failures |
| `src/usage.txt`, `src/setup.txt` | Exact CLI help and onboarding templates |

Storage modules enforce different boundaries: `records` decodes/encodes bytes,
`log` owns record-aligned scans and appends, `tree` owns summary reads/invalidation,
and `lock` owns cross-process exclusion. Writes repair tails and sync before
acknowledgement. Recall borrows canonical records from the reusable input buffer
and copies only matches into its bounded output window.

Public-facing output stays buffered until the CLI boundary, including output
produced before a wake refusal. Filesystem error attribution and newline
translation retain the reference behavior. `AGENTS.md` records the readability
and verification standards for future changes.

```sh
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
cargo test --release --locked
cargo doc --locked --no-deps --document-private-items
```

See [refactor validation](docs/readability-refactor.md) for completed checks and
the measured performance tradeoff against the saved original binary.

## Deliberate differences

`tests/differences.rs` pins down each of these differences.

1. **Regex engine.** `recall` uses the Rust `regex` crate, not Python `re`.
   It stays case-insensitive. It matches the whole record line, including
   the ID and the date, and it does not search the tree summaries. The
   differences are:
   - The crate does not support lookaround (`(?=`, `(?!`, `(?<=`, `(?<!`) or
     backreferences (`\1`). `recall` rejects them with `bad regex: ...` and
     exit status 1. There is no fallback to Python.
   - The crate reads POSIX classes such as `[[:digit:]]` as classes. Python
     reads them as a set followed by a literal `]`.
   - Error text after `bad regex: ` is the crate's text, and it can take
     more than one line.
   - Other crate behavior: `\d`, `\w`, `\s` and `\b` are Unicode-aware, as
     in Python. Case-insensitive matching uses simple case folding. `\Z` is
     an error (use `\z`). Inline flags are `i`, `m`, `s`, `U`, `u`, `x` and
     `R`. Conditionals `(?(1)a|b)` and atomic groups `(?>a)` are errors.
     The crate accepts `a++` as a repeated repetition. Python 3.9 rejects
     it, and Python 3.11 and later read it as possessive.
2. **Only ASCII digits in numbers.** In Python, `str.isdigit()` and `\d`
   also accept other Unicode digits, such as `٠-٣`. The native tool accepts
   only `0`–`9` in block IDs, wake parts, snapshots and config values.
   Numbers that do not fit in 64 bits are rejected with the same messages as
   other malformed values.
3. **Corrupt input is an error, not a traceback.** The native tool rejects a
   non-UTF-8 `config` file with `<path> is not UTF-8 text.`. It rejects a
   log record that is not UTF-8, or that has no readable ID, with
   `Memory #N in LOG.txt is corrupt.`. It rejects arguments that are not
   UTF-8. Python stops with a traceback in all of these cases.
4. **A torn summary record is not a summary.** A crash can leave an
   incomplete trailing record in a level file. Python prints those bytes as
   the summary. The native tool treats the block as not built yet, so the
   next nap repairs the file and rebuilds the block. Readers of `LOG.txt`
   already worked this way in both implementations. The native `recall`
   also stays record-aligned when a note lands during the scan.
5. **Small platform details.** For a filesystem error, the message after
   `<path>: ` is the OS text without Rust's `(os error N)`. On Windows,
   that text comes from `FormatMessage` and can differ from Python's C
   runtime text. On Windows, the lock waits with no limit. Python gives up
   after about 30 s of contention. On macOS, `fsync` is `F_FULLFSYNC`,
   which is more durable than Python's `os.fsync`.
6. **`~user` paths.** On Unix, `~user` in `MEMORY_DIR` is resolved through
   `getpwnam`, as in Python. On Windows, the native tool uses the same rule
   as Python's `ntpath`, but only `USERPROFILE`, `HOMEDRIVE`, `HOMEPATH` and
   `USERNAME` are read.

## Performance

Measured on an Apple Silicon Mac, release build, warm file cache. Each value
is the median of 5 complete CLI runs after one warm-up run. Peak RSS is from
`/usr/bin/time -l`. For each row, `bench/bench.py` first checks that both
implementations give identical output.

| Records | Command | Python | Native | Speedup |
| ---: | --- | ---: | ---: | ---: |
| 1,000 | `recall elixir` | 26.9 ms | 3.5 ms | 7.6x |
| 100,000 | `recall elixir` | 138.2 ms | 10.7 ms | 12.9x |
| 100,000 | `recall topic-(1\|22\|33) ` | 137.6 ms | 12.5 ms | 11.0x |
| 1,000,000 | `recall elixir` | 1146.2 ms | 74.4 ms | 15.4x |
| 1,000,000 | `recall <absent literal>` | 1073.1 ms | 69.9 ms | 15.4x |
| 1,000,000 | `recall .` | 1117.5 ms | 84.1 ms | 13.3x |
| 1,000,000 | `recall ünïcödé` | 1075.0 ms | 70.7 ms | 15.2x |
| 1,000,000 | `recall topic-(1\|22\|33) ` | 1189.1 ms | 99.1 ms | 12.0x |
| 1,000,000 | `wake` | 31.5 ms | 4.5 ms | 7.0x |
| 1,000,000 | `nap` (prompt only) | 25.8 ms | 3.3 ms | 7.8x |
| small | `note` (with fsync) | 26.2 ms | 6.9 ms | 3.8x |

Peak RSS during `recall` at 1M records: about 4 MB for the native tool and
16.5–19 MB for Python. The native tool reads in fixed record-aligned buffers.
The result buffer is limited by `PART_CHARS`. The release binary is about
1.4 MB. To reproduce:

```sh
cargo build --release
python3 bench/bench.py <scratch-dir> 1000 100000 1000000
```

This repository does not include cold-cache, Linux or Windows measurements
yet. An FFF-based recall was not tried. The direct scanner already exceeds
the target by a large margin.

## Local installation and rollback

The user authorized installation on this Apple Silicon Mac. The release binary
was staged and atomically renamed over `~/.optmem/memo`, with the original
Python CLI preserved in a unique backup directory. The installed executable
passed lifecycle, mixed-writer, and real-store read/write checks. No migration
was needed. Native Linux/Windows execution remains unverified.

To restore the previous Python executable atomically:

```sh
python3 ~/.optmem/backups/memo-20261021T060102Z-glenrl3a/restore.py
```

Both implementations share the store format; restoring the executable retains
memories written by the native binary. Details and test artifacts are listed in
[the installation record](docs/local-installation.md).
