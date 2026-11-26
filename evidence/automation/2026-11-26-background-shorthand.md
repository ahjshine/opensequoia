# Public automation run — single-colour background shorthand

Date: 2026-11-26

## Scope and provenance

- Selected exactly one existing, reviewed development commit: `680ee9e17a3208ba433549d6ad761c11ff25952c` (`css: accept single-colour background shorthand`), parent `1fcca4923e5345430e3a28e22d431141ead368ea`.
- Public base and cached `origin/main`: `2b283179797b8e15d75c60309903677181e3c97b`.
- Source `codex/foundation` remained clean and stable at `f15204a47b5e1c428703d275d8e00123fadbea79`; later CSS commits and unrelated crew work were excluded.
- Four unrelated modified app/process files and 99 unrelated untracked cancellation/sandbox evidence files were present in the public clone. At the user's explicit direction, only the 18 staged CSS files plus this report are published; those unrelated files remain untouched and uncommitted, and the clone is intentionally not clean after publication.
- Exported the exact staged index with `git checkout-index` into a fresh, non-repository source directory under `target`. All validation ran against that clean export, not the mixed working tree. No new repository or branch was created.
- The development checkout and every crew worktree remained read-only. Public MIT licensing and all historical evidence notices were preserved; no manifest or lockfile changed.

## Selected files

```text
TASKS.md
crates/engine/src/css.rs
crates/engine/tests/css_subset.rs
docs/reviews/background-fill.md
docs/reviews/border-edges.md
docs/testing/css-subset.md
evidence/verification/background-final-app-smoke.log
evidence/verification/background-final-background-inspect.log
evidence/verification/background-final-background-smoke.log
evidence/verification/background-final-clippy.log
evidence/verification/background-final-inspect.log
evidence/verification/background-final-process-smoke.log
evidence/verification/background-final-release-build.log
evidence/verification/background-final-tests.log
evidence/verification/background-final-verification.json
evidence/verification/background-final-window-smoke.log
evidence/verification/tree3-background-shorthand.log
fixtures/background.html
```

This run report is also included in the publishing commit.

## Checks actually run

All Cargo and executable checks ran in the clean index export with isolated `TEMP` and `TMP` directories beside it under `target`.

- `cargo fmt --all -- --check`: passed.
- `cargo test -p opensequoia-engine --test css_subset --locked`: 23 passed, 0 failed, 0 ignored.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: passed.
- `cargo test --workspace --locked -- --test-threads=1`: 99 passed, 0 failed, 0 ignored across unit/integration tests; doc-tests had no tests.
- `cargo build --release --locked`: passed.
- `target/release/opensequoia.exe --inspect fixtures/background.html`: passed; 25 nodes, 39 text runs, no unsupported elements or CSS, and the documented limited-layout diagnostic remained visible.
- `target/release/opensequoia.exe --process-smoke`: passed; healthy, malformed-rejection and restart paths completed with three children spawned and reaped.
- `target/release/opensequoia.exe --smoke-test fixtures/background.html`: passed; `paints=1`, `drawn_text_runs=34`, `child_rendered=1`.
- `cargo metadata --locked --no-deps --format-version 1`: all three workspace packages report MIT.
- `git diff --cached --check`: passed.
- All 18 selected files match source revision `680ee9e17a3208ba433549d6ad761c11ff25952c` and the exported validation files match the staged index blob-for-blob.
- Selected-artifact audit: 18 source files plus this report, no binary or executable payload, no file over 100 MiB. The unrelated 4 modified and 99 untracked files are explicitly excluded.
- Credential scan covered common private-key, GitHub, AWS, Slack, bearer, password, token, authorization and cookie patterns: no matches. No personal browsing data was selected.
- No test/lint suppression, new unsafe block, sandbox/process-policy change, remote-content enablement, input-restriction relaxation, or IPC change was added.

## Limitations

- The source review in `docs/reviews/background-fill.md` was excerpt/snippet-based; root inspection confirmed its two out-of-snippet assumptions. This automation run validates the integrated public snapshot but is not a new independent security or parser review.
- Only one colour token or `none` is accepted by this background shorthand. Multi-token, image, gradient, alpha, function and multi-layer values remain unsupported and rejected.
- The native smoke executes the GDI background/border fill paths but does not compare pixels. No CSS conformance, full website compatibility, accessibility, performance, signing, sandbox, origin-authentication, remote-browsing, or production-release verdict is claimed.
- Historical evidence retains its original environment, source hashes, limitations and notices. Existing project gaps in `TASKS.md` remain open.
- Unrelated working-tree changes are not validated, committed, or cleaned by this run; their presence is an explicitly user-accepted exception to the usual clean-clone finish.

## Outcome

Validation of the isolated staged snapshot passed. This single coherent CSS batch is eligible for one descriptive publishing commit and a subsequent fast-forward push. This pre-push record does not claim that the push or remote verification has completed.
