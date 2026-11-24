# Initial public automation import — 2026-11-24

## Scope and provenance

- Imported the committed development snapshot `77992419cda31b9116831bb521d7bd025e90c07f` from the read-only local development checkout.
- Preserved the GitHub `main` parent `aa875a5e39328ce955f42e77dd5c246f4a27ac63` and its initial MIT license decision.
- Did not modify the active development checkout or any crew worktree.
- Current workspace package metadata and newly generated package notices are MIT. Historical artifacts and notices remain as recorded, including MPL-2.0 references tied to historical packages.

## Checks performed

- `cargo fmt --all -- --check`: passed.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: passed.
- `cargo test --workspace --locked -- --test-threads=1`, with `TEMP` and `TMP` isolated under `target/automation-temp`: **90 passed, 0 failed, 0 ignored** across unit/integration tests; doc-tests had no tests.
- Python AST parsing of `scripts/package_dev.py`: passed.
- `cargo metadata --locked --no-deps --format-version 1`: all three workspace packages report MIT.
- Common GitHub/AWS/private-key/token patterns: no matches in the staged snapshot.
- Staged snapshot before this report: 464 files, approximately 9.8 MB; no file exceeded 100 MiB.
- `git diff --cached --check`, excluding only `evidence/verification/i03a-runtime.log`: passed. That historical log has three trailing-whitespace/bare-CR warnings and is preserved byte-for-byte as evidence.

## Environment diagnosis

The default parallel test run first hit restricted-child-PowerShell behavior and then shared temporary/helper-file failures. A focused process-limit test passed in an unrestricted Windows session. The complete suite subsequently passed with isolated temporary files and serialized test execution. No test was skipped or weakened.

## Limits

This was a source-import validation, not a new independent review of every historical evidence record. No new GUI, packaging, AppContainer, network, compatibility, performance, signing, or production-release verdict is claimed. Existing project gaps in `TASKS.md` remain open.
