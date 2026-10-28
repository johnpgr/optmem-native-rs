# Scope

This is the Claude-derived native OptMem CLI. Keep changes in this directory
unless the user asks otherwise. Tests and benchmarks must use disposable stores:
standalone probes set `MEMORY_DIR` explicitly; the existing differential harness
isolates HOME/USERPROFILE and its default store. Installation over `~/.optmem/memo`
is a separate user action.

# Readability

- Separate validation, work, and output with blank lines. Leave vertical space
  between methods, declarations, and logical blocks.
- Use focused modules and functions with clear ownership. Keep CLI/process I/O,
  command behavior, configuration, compression planning, and storage separate.
- Prefer guard clauses, early returns, and continue to deeply nested happy paths.
- Give state descriptive fields. Use named byte/line limits and dated-memory input
  rather than positional configuration indices or date/text tuples.
- Document module responsibilities and item contracts. Explain storage layout,
  locks, durability, snapshots, byte budgets, and compatibility exceptions where
  the implementation relies on them. Comments should explain reasons.
- Preserve CLI output, exit statuses, on-disk bytes, and documented differences.
  Keep recall per-record and bounded; preserve the padding/canonical-record fast path.
- Keep dependencies lean. FFF is outside this short-lived CLI's scope, as decided
  by the user; no persistent search/index process is needed.

# Verification

Run formatting, Clippy with warnings denied, and all Rust tests. For structural
changes, also build release, run the CLI suites against release with
`cargo test --release --locked`, and generate private-item documentation with
`cargo doc --locked --no-deps --document-private-items`.

Compile-check Linux and Windows when their targets are available. Compilation
does not establish native runtime compatibility. If the scanner or ownership
layout changes, compare release behavior and representative recall performance
against a saved pre-change binary on the same disposable store.
