# S04a tree-construction handoff

Task: S04a / leaf 1.1.1, lane `engine-conformance`, branch `crew/s04a`. This is a local candidate for controller review, not formal READY, /ship, production compatibility, or browser parity.

## Delivered

- Completed the existing flat-arena retention work: doctypes, comments, processing-instruction storage, namespaces, and detached template contents are available to bounded canonical inspection without changing the limited renderer's exclusion rules.
- Added iterative `Document::tree_dump` with sibling/link validation, cycle checks, UTF-16 attribute ordering, and a 4 MiB output bound. Deep input does not use recursive traversal or recursive arena drop.
- Added real document/fragment options with explicit parser scripting mode. JavaScript is never executed. The application's `Document::parse` default remains scripting-off.
- Completed the actual `.dat` adapter with independent case subprocesses, per-case deadlines, explicit pass/fail/unsupported/skip/timeout/crash/harness_error accounting, and complete case identity/mode records. Scratch replies use create-new files under `target/tree3`; stale or malformed replies are not accepted.
- Corrected DAT CRLF handling against the pinned upstream runner. A final isolated carriage return is preserved. Genuine bounded errors and mismatches remain `fail`.
- Added an independent Python acceptance oracle, immutable-by-default source inventory, exact report census, summary reconciliation, and eight negative controls. The upstream ignore file is inspected and hashed but never applied.

## Files changed

Owned implementation and tests:

- `crates/engine/src/dom.rs`
- `crates/engine/src/layout.rs`
- `crates/engine/src/lib.rs`
- `crates/engine/src/tree_dump.rs`
- `crates/engine/examples/support/dat.rs`
- `crates/engine/examples/tree_construction.rs`
- `crates/engine/tests/dat_format.rs`
- `crates/engine/tests/tree_inspection.rs`

Owned evidence and documentation:

- `docs/testing/tree-construction.md`
- `evidence/verification/tree3-acceptance.py`
- `evidence/verification/tree3-report.jsonl`
- `evidence/verification/tree3-suite-hashes.json`
- `evidence/verification/tree3-handoff.md`
- `evidence/verification/tree3-quality.log`
- `evidence/verification/tree3-acceptance.log`

`crates/engine/tests/static_document.rs` was verified byte-identical during migration and not edited. No manifest, lockfile, app/IPC, root record, `.unlazy`, legacy-checkout, or gate-ledger edit was made. No manual gate is self-attested.

## Source pins and evidence hashes

- WPT suite revision: `31d1feb70e2d02f25ae26254caba19c8de0b56e0`.
- Inspected html5ever adapter checkout: `492f52a02fd07c2321e59ccff90038e26b9c90bc`.
- Actual locked html5ever: `0.40.1`, registry VCS `d7232d746d3112f08926d169591576bd12fe2fdf`.
- Parser archive SHA-256: `456a1a377e608e555d22ddab27ac0114bc7a7b4199078108e34c2aeae6c9b130`, matching `Cargo.lock`.
- `tree3-suite-hashes.json`: 68 independently hashed inputs, SHA-256 `563bfe0fd1c9f450f0395d621127434dd685ecbabe514e34547f5069243ef465`.
- `tree3-report.jsonl`: SHA-256 `f51358241cb9fef2672be8c30974bcf8e10e03f4667d8dd8e8b8e7d65f6f4ca3`.
- `tree3-acceptance.py`: SHA-256 `0c55b6b96e49370f8de0cd96d286f43f91eef364bd38098d63c38cdf121eb128`.
- `tree3-quality.log`: SHA-256 `75f5bbee091b1dbb10c2d910805a3bdebad724602d97c2c9e2906e273eda0cd1`.
- `tree3-acceptance.log`: SHA-256 `36d6b4576dabfce637c7dc53d1e411aa7228fcdc63e0dcf67e0cd886acebdf3d`.

The migration audit is `target/tree3/migration-audit.json`: nine owned source files matched legacy bytes exactly, zero mismatches. Cargo binaries were rebuilt rather than migrated.

## Commands and actual results

### Migration and focused defect controls

- Initial inherited engine suite: 19 tests passed before the DAT edits.
- Initial draft corpus: 59 files, 1,779 source cases, 3,519 outcomes; 166 pass, 3,349 fail, 4 unsupported, zero operational failures. Investigation showed mass CRLF expected-tree/fragment mismatches, not a basis for hiding failures.
- New CRLF regression was observed RED: retained `\r\n` differed from normalized `\n`. After the repair, the focused DAT suite passed.
- Domain reread added a final-unterminated-CR regression and observed it RED (`left: "| \"x\""`, `right: "| \"x\"\r"`). Both Rust and Python parsers then passed the focused check.
- First post-CRLF full corpus rerun: 3,511 pass, 4 fail, 4 unsupported, zero skip/timeout/crash/harness_error.
- Focused tree inspection, including the UTF-16 astral/BMP attribute-order case: 6/6 passed. Final DAT suite: 4/4 passed.

### Final quality log

`tree3-quality.log` contains these exact fresh commands and outputs:

```text
cargo fmt -p opensequoia-engine -- --check
cargo clippy --target-dir target/tree3 -p opensequoia-engine --all-targets --locked -- -D warnings
cargo test --target-dir target/tree3 -p opensequoia-engine --locked
```

Result: all passed; 20 tests passed, zero failed, zero ignored. An earlier focused Clippy run found two argument-iteration lints; they were corrected and the final run is clean.

### Final corpus acceptance

```text
python evidence/verification/tree3-acceptance.py
```

Result from `tree3-acceptance.log`:

```text
TREE3_SOURCE_INVENTORY_OK files=68 dat_files=59 source_cases=1779 outcomes=3519
TREE3_ADAPTER_BUILD_OK
TREE3_ADAPTER_EXIT=1 (genuine fail/unsupported outcomes remain nonzero)
TREE3_OUTCOMES pass=3511 fail=4 unsupported=4 skip=0 timeout=0 crash=0 harness_error=0
TREE3_NEGATIVE_CONTROLS_OK count=8
S04A_ACCEPTANCE_OK
```

Adapter exit 1 is intentional while genuine `fail` or `unsupported` outcomes exist. Acceptance separately proves operational health and report integrity rather than treating process exit 1 as success by itself.

## Corpus failures and limits

The 59-file suite contains 1,779 source cases and 3,519 scripting-specific outcomes.

- Four failures: `webkit02.dat#44@off`, `webkit02.dat#44@on`, `webkit02.dat#45@off`, `webkit02.dat#45@on`. Expected option content is cloned under `selectedcontent`, but actual output lacks that clone. The mismatch offset is 99 in each report row. This is consistent with the unimplemented html5ever selectedcontent sink hook. Upstream's two ignored source indices are deliberately not ignored here.
- Four unsupported: `scripted_adoption01.dat#0@on`, `scripted_ark.dat#0@on`, `scripted_webkit01.dat#0@on`, `scripted_webkit01.dat#1@on`. These require JavaScript during parsing and were not executed.
- Zero skip, timeout, crash, or harness_error outcomes occurred. No failed-case rerun or flake classification was used.

Only tree output is compared. Parse-error lists, JavaScript, live DOM behavior, full WPT coverage, modern website compatibility, performance, and production sandboxing remain outside this evidence. Fragment parsing has no ancestor form element, contextual attributes, or browser host APIs. A crash/timeout would block acceptance and require investigation; healthy-run child stderr is not retained.

## Review passes and test value

1. Completed the inherited implementation and real corpus adapter without resetting uncommitted work.
2. Domain-expert reread grounded serializer/DAT behavior in the pinned upstream sources, exposing and repairing the final-CR discrepancy and adding UTF-16 ordering coverage.
3. Correctness/security/integration/performance review checked arena bounds, stale replies, scratch ownership, source mutation during runs, downstream error matching, output caps, and report accounting. This added bounded DAT reads, exact inventory counts, Windows path normalization, and exact stdout reconciliation. No performance result is claimed from the correctness suite.
4. Low-cost polish added value cards, formatting, Clippy fixes, reproducible documentation, and negative controls. The final full improvement pass found no remaining adapter/oracle defect requiring another edit.

Every new/changed Rust test has a nearby `Value: protects=...; fails_when=...; why_new=...; seam=...; no_claim=...` card in `dat_format.rs`, `tree_inspection.rs`, or the `dom.rs` unit test. The two DAT repairs have explicit observed RED/GREEN records above. No test was skipped or weakened.

## Controller action

Review and integrate the full-SHA lane commit named in `opensequoia-crew/reports/s04a.md`; run independent source/report review and integrated workspace gates. Keep the four selectedcontent failures visible in the engine backlog. Do not convert them to skips and do not describe this bounded tree-construction result as blanket WPT compatibility or formal READY.
