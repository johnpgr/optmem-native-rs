# Local native installation

Installed and tested on 2026-10-21 on Apple Silicon macOS, at the user's request.

- Executable: `~/.optmem/memo`, a Mach-O arm64 binary, mode 0755.
- Source: `~/dev/optmem-native-claude/target/release/memo`, rebuilt with
  `cargo build --release --locked`.
- Native SHA-256: `2c76d915ae9058496d1a66e109c265d3ec8621d3cae9df2cbc1185b4aa9d33d5`.
- Original Python backup: `~/.optmem/backups/memo-20261021T060102Z-glenrl3a/memo.py`.
- Python SHA-256: `3dc120d01be3115ef6267eab4103e7909fc830d6227b549f20991ba999ee9ffb`.

The installer copied and verified the Python backup, staged the native binary
beside the destination, set executable permissions, synced it, and atomically
replaced the CLI. Existing memory records and summaries required no migration.

## Installed executable checks

All checks invoked the installed `~/.optmem/memo`, not just the build artifact.

- 21 lifecycle invocations on disposable stores exercised all nine commands,
  including historical import, Unicode notes, compression, pagination, recall,
  configuration/reset, zoom, forget/rebuild, and invalid input. Output, status,
  and store bytes matched the Python backup after path normalization.
- 12 concurrent mixed Python/native writers on one disposable store received
  unique IDs 0–11, produced aligned 320-byte records, and gave identical recall.
- Native and Python wake, config, recall, zoom, and nap matched on the actual
  21-record memory store. Those read-only checks left store bytes unchanged.
- The native CLI then saved the real installation memory as #21 and compressed
  #20–21 through `nap`. Both engines read the resulting data identically. Wake
  succeeded with 22 records, and nap reported no pending compressions.

The source's 33 debug/release tests, strict formatting/Clippy/Rustdoc checks,
and Linux/Windows compile checks had passed before installation. Native runtime
testing on Linux and Windows remains outstanding.

## Rollback

```sh
python3 ~/.optmem/backups/memo-20261021T060102Z-glenrl3a/restore.py
```

This stages and atomically restores the saved Python executable. It does not
change the memory store; Python can read memories written by the native CLI.

The backup directory also contains `manifest.json` and `installation-tests.json`.
Copies of these records live in `target/refactor-review/` for local development.
