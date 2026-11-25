# Public automation run — 1–3 token border shorthand

Date: 2026-11-25

## Scope and provenance

- Selected one coherent batch of two adjacent existing source commits:
  - `b7e623f0354bb66924eb81803cd22c75401ce25f` (`css: accept 1-3 token border shorthand`), parent `9aa62777a282d49383cebe04dfb96aceada1fce1`.
  - `1fcca4923e5345430e3a28e22d431141ead368ea` (`docs: link border shorthand evidence`), which only completes the same batch's `TASKS.md` evidence links.
- Public base and fetched `origin/main`: `58755a72a1fc1ff532912cb65ec7f35145a6c345`.
- Source `codex/foundation` remained clean and stable at `f15204a47b5e1c428703d275d8e00123fadbea79`; later CSS commits and unrelated active crew work were excluded.
- Applied both source commits without committing, then validated the public clone. The development checkout and every crew worktree remained read-only.
- Preserved the public MIT license, root package metadata, README licensing wording, and all historical evidence notices. No manifest or lockfile changed.

## Selected files

```text
TASKS.md
crates/engine/src/css.rs
crates/engine/tests/css_subset.rs
docs/reviews/border-edges.md
docs/reviews/css-subset.md
docs/testing/css-subset.md
evidence/verification/border-final3-app-smoke.log
evidence/verification/border-final3-background-inspect.log
evidence/verification/border-final3-background-smoke.log
evidence/verification/border-final3-clippy.log
evidence/verification/border-final3-inspect.log
evidence/verification/border-final3-process-smoke.log
evidence/verification/border-final3-release-build.log
evidence/verification/border-final3-tests.log
evidence/verification/border-final3-verification.json
evidence/verification/border-final3-window-smoke.log
evidence/verification/tree3-border-shorthand.log
fixtures/background.html
```

This run report is also included in the publishing commit.

## Checks actually run

All Cargo and executable checks used isolated `TEMP` and `TMP` directories under `target/automation-temp`.

- `cargo fmt --all -- --check`: passed.
- `cargo test -p opensequoia-engine --test css_subset --locked`: 21 passed, 0 failed, 0 ignored.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: passed.
- `cargo test --workspace --locked -- --test-threads=1`: 97 passed, 0 failed, 0 ignored across unit/integration tests; doc-tests had no tests.
- `cargo build --release --locked`: passed.
- `target/release/opensequoia.exe --inspect fixtures/background.html`: passed; 25 nodes, 39 text runs, no unsupported elements or CSS, and the documented limited-layout diagnostic remained visible.
- `target/release/opensequoia.exe --process-smoke`: passed; healthy, malformed-rejection and restart paths completed with three children spawned and reaped.
- `target/release/opensequoia.exe --smoke-test fixtures/background.html`: passed; `paints=1`, `drawn_text_runs=34`, `child_rendered=1`.
- `cargo metadata --locked --no-deps --format-version 1`: all three workspace packages report MIT.
- `git diff --cached --check`: passed.
- Staged-artifact audit: 18 source files plus this report; no binary or executable payload, no file over 100 MiB, no untracked or unstaged source files.
- Credential scan covered common private-key, GitHub, AWS, Slack, bearer, password, token, authorization and cookie patterns: no matches. A local development build path occurs only in historical verification evidence; no personal browsing data was selected.
- All 18 selected files match source revision `1fcca4923e5345430e3a28e22d431141ead368ea` blob-for-blob. Public-only MIT licensing differences remain outside this batch.
- No test/lint suppression, new unsafe block, sandbox/process-policy change, remote-content enablement, input-restriction relaxation, or IPC change was added.

## Limitations

- The source review in `docs/reviews/border-edges.md` was limited to minimal snippets and did not inspect the repository or full diff. This automation run validates the integrated public snapshot but is not a new independent security or parser review.
- Width-only `border: Npx` retains the documented solid-width subset deviation. Omitted shorthand components are not reset at this revision, unlike full CSS shorthand reset semantics; that limitation remains explicit and later source work is not included.
- The native smoke executes the GDI background/border fill paths but does not compare pixels. No CSS conformance, full website compatibility, accessibility, performance, signing, sandbox, origin-authentication, remote-browsing, or production-release verdict is claimed.
- Historical evidence retains its original environment, source hashes, limitations and notices. Existing project gaps in `TASKS.md` remain open.

## Outcome

Validation passed and this single coherent batch is eligible for one descriptive publishing commit. This pre-push record does not claim that the subsequent fast-forward push or remote verification has completed; those steps remain mandatory.
