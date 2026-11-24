# R01 independent foundation review

Reviewer: brave_baseline agent, separate from engine/app authors. Date: 2026-11-22. Scope: SPEC/CONSTRAINTS/ARCHITECTURE/TASKS and baseline/parity/bloat/security/benchmark records; S01 engine/app source; Cargo graph; Windows resources/build tooling. This is an independent same-model agent review, not an independent model or formal security proof.

Status: **S01 source review accepted for controlled local development; current workspace integration revalidation pending while S02 IPC files are being added.** Full browser/release remains unapproved. No remote browsing/security/parity/performance endorsement.

## Workflows and actual coverage

Read installed gstack `review` and `plan-eng-review` version 1.0.0 instructions, shared preamble `4e82fa4c839f`, checklist, scope/plan-completion and adversarial/review-sections guidance under `C:/Users/deepa/.claude/skill-library/`. Applied evidence calibration, scope/completion audit, critical boundary/enum/gate scan, source/consumer reading, architecture/failure-path review and claim limits. Also used installed native code-review-and-quality and security-and-hardening guidance.

Canonical gstack `bin/gstack-skill-start`, review logger/gate-diff/audit helpers are absent. No helper result, review receipt, hook/telemetry or outside-model review is fabricated. Documented degraded mode permits substantive work, but the full automated gstack workflow was not run. No PR/base commit exists at this initial unborn branch; reviewed all non-ignored new S01 source directly against the authoritative mission and acceptance contract. No fixtures were executed as instructions; they are test data.

## Findings and dispositions

| ID | Evidence and impact | Disposition |
|---|---|---|
| R01-F1 | Initial `input.rs` validated path spelling then called `File::open`. Local-looking symlink/junction or mapped-drive paths could traverse network shares despite documentation. Static concrete Windows path-resolution issue, confidence 9/10. | Author added path revalidation in load, device variants, stable local-volume classification before metadata, ancestor reparse rejection, final `FILE_FLAG_OPEN_REPARSE_POINT`, real local-junction regression and explicit TOCTOU limitation. Reviewed corrections. Controlled-file boundary mitigated; drive remap/ancestor races remain production blockers, not sandbox guarantees. No network-share probe was executed. |
| R01-F2 | FEATURE_PARITY initially cited `browser/devtools`, `browser/extension_system/brave_extension_system.cc`, `services/brave_sync`, which fail `git cat-file -e HEAD:<path>` at pinned core revision. Confidence 10/10, documentation evidence defect. | Root notified to replace with actual `browser/extensions/BUILD.gn`, `components/brave_sync/BUILD.gn`, `components/sync/service/brave_sync_auth_manager.cc` and explicit inherited pinned Chromium DevTools reference. Does not change retained scope. |
| R01-F3 | Dependency unsafe is not covered by engine crate `forbid(unsafe_code)`; html5ever has SIMD/unchecked slice code and tendril/string_cache/parking_lot use unsafe. | Documented in discovery/security records; focused parser SIMD review below. No concrete unsoundness found in this bounded review. Full dependency audit/fuzz/sanitizer work remains mandatory before release. |

No additional concrete S01 unsafe/lifetime/reentrancy or arena/layout bounds defect was identified after the corrections. This is a scoped null result, not a safety guarantee. IPC/network/origin storage/extension/update implementation has not yet been reviewed.

## Architecture and code review

Engine uses a flat arena and iterative traversal, stable parser handles and bounded input/nodes/diagnostics/layout. HTML is parsed by actual html5ever tree construction, not regular expressions or OS browser interpretation. Overflow rejects incomplete results. Layout is explicitly approximate block/text flow and diagnoses unsupported CSS/features; this is not full CSS/DOM/rendering parity. Unit cases exercise Unicode/entity handling, recovery, foster parenting, hidden content, narrow viewports, malformed generated sequences, input/node overflow and deep stack-safe ownership. Generated examples are finite sampling, not a completed fuzz campaign or standards suite.

Windows module has thread-local owned state and releases borrow guards before OS calls. Paint consumes an Rc-owned display snapshot; no HWND stores a Rust pointer and no page/message pointer is dereferenced. Reviewed CreateWindow/event-loop/teardown, callback panic containment, GDI text buffer lifetime, font selection/restoration/deletion, BeginPaint/EndPaint pairing, signed DPI coordinate arithmetic and scroll bounds. Resource generation calls Windows SDK `rc.exe` using structured arguments; `.rc`/manifest provide OpenSequoia branding, prerelease version, asInvoker/no UIAccess, DPI and long-path metadata. This is allowed non-Rust build tooling and OS drawing, not native browser engine code. Final GPU/font shaping/UIA/IME/selection/high contrast and high-resolution scrolling still need real integration/compatibility checks.

The full process/site isolation/sandbox/secure IPC/TLS/origin/credential/storage/extension/update architecture remains proposed. S01 performs controlled local rendering only. No relevant requirement was declared completed or removed merely to improve size/memory; baseline comparison of this subset would be invalid. Root source/artefact identity and exploratory idle snapshot limits are documented; signed installer provenance and official GN/PGO build inputs remain unknown.

## Dependency and unsafe review

Executed target-aware `cargo tree --workspace --locked --target x86_64-pc-windows-msvc -e normal,build` and `cargo metadata --locked --format-version 1`. Exact direct parser/binding pins are html5ever 0.40.1 and windows-sys 0.61.2; lockfile pins transitive versions/checksums. Captured graph/inventory under `evidence/verification/review-dependency-*`. Per-package extensions searched: C/C++ headers/sources, assembler, DLL/lib/object/archive, excluding tests/examples. No such bundled implementation file appeared in this searched production source boundary; absence here does not prove final executable composition. Windows-link 0.2.1 imports OS DLL entrypoints via raw-dylib macros. Windows target does not build target-specific libc/redox packages merely because they appear in lockfile.

Read web_atoms build script (Rust entity/atom table generation), parking_lot_core sanitizer cfg script, quote rustc-version probe and selected proc-macro2 build/probe paths. No selected native browser compiler/provider identified. Build/development tools may use native executables; generated resources carry metadata, not HTML/CSS/JavaScript implementation. License inventory distinguishes MIT/Apache/Unlicense/Unicode terms; release packaging still requires actual license texts/attributions and updated SBOM.

Focused html5ever tokenizer SIMD review at registry `html5ever-0.40.1/src/tokenizer/mod.rs:756,2915-3065`: CPU detection gates target features; 16-byte reads require `i + STRIDE <= raw_bytes.len()`; residual scan stops at ASCII sentinel or the full tendril length, both valid UTF-8 boundaries before unchecked sub/pop operations. Initial input is `&str` and chunk boundaries are checked. No concrete bounds defect found on the inspected x64 path. Upstream SSE2 safety prose says NEON while actual target feature is SSE2: documentation typo, not evidence the CPU guard is wrong. Arm SIMD runtime was not executed. Tendril/string-cache internal pointer/tag/allocation invariants and memchr/locking internals were inventoried only, not exhaustively proven. Advisory database scan, formal verification, Miri/sanitizers and fuzz campaigns were not run by R01.

## Reproducible verification

Environment: Windows 11 x64, exact Rust/Cargo 1.98.1 toolchain restored by root. Recorded source fingerprints in `evidence/verification/review-snapshot.json`; ongoing integration can make a fingerprint stale, so the final root check must bind its consumed inputs.

* Independent `cargo test --workspace --locked`: **exit 0**, app **6** tests and engine integration **9** tests passed; **0 ignored**. Real UTF-8/file limit and local Windows directory-junction behavior are included. Log `review-workspace-tests.txt`.
* Independent `cargo clippy --workspace --all-targets --locked -- -D warnings`: **exit 0**. Log `review-clippy.txt`.
* Subsequent `cargo fmt --all -- --check` attempt: **exit 1**, metadata could not load newly added `crates/ipc` because its in-progress manifest had no target. Log `review-format.txt`. This is a real current-workspace failure, not an S01 formatter verdict. Headless run chained after it **was not executed**.
* Windows-author smoke evidence was read as reused author evidence only, not independently executed by R01. Actual visual UI/resize/scroll/keyboard and final resource verification belong to root; no screen-reader/page accessibility pass claimed.

Tests and Clippy above covered the S01 graph before the IPC addition. Do not cite them as proof the new IPC/current workspace passed. No failing criteria, skips, warnings or time budgets were loosened by this reviewer. Full WPT/Test262/Wasm corpus/adapters remain missing; no modern website compatibility or performance budget is satisfied by these checks.

## Handoff and acceptance

R01 provided actionable early findings, inspected their corrections, independently executed meaningful S01 tests/Clippy and preserved missing coverage. S01 code/dependency/unsafe review accepted within controlled local scope; P01 reference correction and current-graph format/test/build confirmation remain for root integration. No release approval. Next: finish IPC target, re-run current workspace quality gates, re-read any changed S01 source against fingerprints, perform actual native UI/version/artifact checks, then review the isolated renderer protocol/lifecycle before enabling any network or profile access. Keep mapped-volume/ancestor TOCTOU, native dependency unsafe, missing sandbox, unsupported standards, unlicensed DRM/service availability and incomplete Craft internals as visible blockers.

Operational learning: path-spelling validators cannot establish local filesystem access; volume type and reparse behavior must be verified independently, with races explicitly scoped. Canonical imported gstack instructions are available while their runtime helpers are absent; substantive review cannot produce a valid missing logger receipt.
