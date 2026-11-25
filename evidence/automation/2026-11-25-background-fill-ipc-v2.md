# Public automation run — block background fills and IPC v2

Date: 2026-11-25

## Scope and provenance

- Selected exactly one existing, reviewed development commit: `ad0f70df62e2c301502cc35d1112457c8730273c` (`engine: paint block background fills over IPC v2`).
- Its parent is `77992419cda31b9116831bb521d7bd025e90c07f`, the development snapshot already represented by public `main`.
- Public base and fetched `origin/main`: `d2f74135ea3ce1509cfa907c1d8819f605f1fdc0`.
- Source `codex/foundation` remained at `f15204a47b5e1c428703d275d8e00123fadbea79`; later CSS commits were intentionally excluded.
- Applied the selected commit without committing, then validated the public clone. The development checkout and every crew worktree remained read-only.
- Preserved the public MIT license, root package metadata, README licensing wording, and all historical evidence notices. No manifest or lockfile changed.

## Selected files

```text
ARCHITECTURE.md
FEATURE_PARITY.md
README.md
TASKS.md
crates/app/src/chrome.rs
crates/app/src/windows.rs
crates/app/src/worker.rs
crates/engine/src/dom.rs
crates/engine/src/layout.rs
crates/engine/src/lib.rs
crates/engine/tests/css_subset.rs
crates/engine/tests/static_document.rs
crates/ipc/src/codec.rs
crates/ipc/src/lib.rs
crates/ipc/tests/framing.rs
docs/reviews/background-fill.md
docs/reviews/css-subset.md
docs/testing/css-subset.md
evidence/verification/fill-final2-app-smoke.log
evidence/verification/fill-final2-background-inspect.log
evidence/verification/fill-final2-background-smoke.log
evidence/verification/fill-final2-clippy.log
evidence/verification/fill-final2-inspect.log
evidence/verification/fill-final2-process-smoke.log
evidence/verification/fill-final2-release-build.log
evidence/verification/fill-final2-tests.log
evidence/verification/fill-final2-verification.json
evidence/verification/fill-final2-window-smoke.log
evidence/verification/tree3-final.log
fixtures/background.html
```

This run report is also included in the publishing commit.

## Checks actually run

All Cargo and executable checks used isolated `TEMP` and `TMP` directories under `target/automation-temp`.

- `cargo fmt --all -- --check`: passed.
- `cargo test -p opensequoia-engine --test css_subset --locked`: 18 passed, 0 failed, 0 ignored.
- `cargo test -p opensequoia-ipc --locked`: 11 passed, 0 failed, 0 ignored; unit/doc-test targets had no tests.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: passed.
- `cargo test --workspace --locked -- --test-threads=1`: 94 passed, 0 failed, 0 ignored across unit/integration tests; doc-tests had no tests.
- `cargo build --release --locked`: passed.
- `target/release/opensequoia.exe --inspect fixtures/background.html`: passed; 25 nodes, 35 text runs, no unsupported elements or CSS, and the documented limited-layout diagnostic remained visible.
- `target/release/opensequoia.exe --process-smoke`: passed; healthy, malformed-rejection and restart paths completed with three children spawned and reaped.
- `target/release/opensequoia.exe --smoke-test fixtures/background.html`: passed; `paints=1`, `drawn_text_runs=34`, `child_rendered=1`.
- `cargo metadata --locked --no-deps --format-version 1`: all three workspace packages report MIT.
- `git diff --cached --check`: passed.
- Staged-artifact audit: 30 source files plus this report; no binary or executable payload, no file over 100 MiB, no untracked or unstaged source files.
- Credential scan covered common private-key, GitHub, AWS, Slack, bearer, password, token, authorization and cookie patterns: no matches. Local development build paths occur only in historical verification evidence; no personal browsing data was selected.
- Twenty-nine selected files match the source commit blob-for-blob. `README.md` additionally retains the public branch's pre-existing MIT and historical-notice wording.
- No test/lint suppression, new unsafe block, sandbox/process-policy change, remote-content enablement, or input-restriction relaxation was added.

## Limitations

- The source review recorded in `docs/reviews/background-fill.md` was excerpt-only; this automation run validates the integrated public snapshot but is not a new independent security review.
- The native smoke executes the GDI fill path but does not compare pixels. No CSS conformance, full website compatibility, accessibility, performance, signing, sandbox, origin-authentication, remote-browsing, or production-release verdict is claimed.
- Historical evidence retains its original environment, source hashes, limitations and notices. Existing project gaps in `TASKS.md` remain open.

## Outcome

Validation passed and this single coherent batch is eligible for one descriptive publishing commit. This pre-push record does not claim that the subsequent fast-forward push or remote verification has completed; those steps remain mandatory.
