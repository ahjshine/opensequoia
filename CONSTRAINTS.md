# OpenSequoia constraints

This contract preserves the user's full mission. Do not relax it in a change that fails it. Missing mechanisms or measurements stay unmet; no arbitrary performance budget is invented.

| Constraint | Required verdict / rationale | Verification mechanism | Gate |
|---|---|---|---|
| Rust engine and application | No embedded forbidden engine or bundled native browser implementation; OS interfaces explicitly documented | locked Cargo tree/source/native-build inspection, SBOM and Craft shipped-artifact analysis | Every dependency and release |
| Safe engine | `#![forbid(unsafe_code)]`; unsafe only isolated Windows boundary with invariants and independent review | compiler, `rg unsafe crates`, review records | Every edit |
| Build quality | zero format or Clippy warnings; meaningful tests all pass, no weakened tests/ignores to achieve pass | `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --locked -- -D warnings`; `cargo test --workspace --locked` | Every slice |
| Dependency integrity | exact direct versions plus Cargo.lock; all runtime licenses/native dependencies reviewed; unresolved advisories block release | `cargo tree --locked`; cargo-deny/cargo-audit when installed; archive checksums and source audit | Dependency change and CI |
| S01 input bound | <=262144 UTF-8 bytes and <=32768 arena nodes; no recursive hostile DOM traversal/drop | parser boundary, hostile input/regression tests | S01 |
| Production isolation | site/process/origin isolation, Windows least privilege sandbox, secure bounded IPC; size/RAM cannot weaken any | process/token/resource/IPC attack tests and security review | Required before remote untrusted browsing |
| TLS/origin/storage | certificate checks, origin keys, permissions and profile boundaries enforced | certificate/origin/permission/corruption negative tests | Before network/persistence |
| Privacy | no browsing data or secrets in evidence; private session persistence rules tested; no optional telemetry | data-flow review and private-session tests | Every feature |
| Compatibility | pinned WPT/Test262/Wasm and corpus report pass/fail/skip/unsupported/regression separately; never remove failing required tests | versioned adapters and reports; authoritative standards/differential review | Subsystem and release |
| Accessibility | full keyboard interaction and applicable Windows accessibility semantics; audited gaps visible | actual application/UI Automation and manual assistive technology tests | UI and release |
| Performance | establish concrete distributions/memory/startup/navigation/frame budgets only after measured Brave baseline; equivalent retained functionality | BENCHMARKS.md reproducible process-tree/workload harness, multiple runs | Unset/unmet until baseline and comparable engine exist |
| Attribution/branding | OpenSequoia metadata/UI; no Brave trademark implying affiliation; required licenses/notices retained | source/license review and shipped-artifact/version inspection | Packaging |
| Release | signed trusted updates, installation/removal/recovery tested, vulnerability response and upstream security intake active | actual Windows installer/update attack tests and incident drill | Release |

External checks (compiler/rustfmt/Clippy and upstream conformance) complement project tests. Coverage, fuzz campaign duration and runtime performance figures are not yet measured; no coverage/security claim is implied by unit tests. Dependency audits, fuzzing and release isolation are required work, not optional waived checks.

Exceptions: none. Narrow S01 functionality is an incremental implementation state, not a scope exception. All full-browser criteria remain required.
