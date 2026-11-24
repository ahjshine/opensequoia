# D02 / S01 engine handoff

Owner: `/root/rust_feasibility`. Selection/run date: 2026-11-22. Root owns integration and independent acceptance. State: **Under review**; engine self-checks passed. Full browser remains incomplete.

## Files

- `docs/discovery/rust-engine.md`: component revisions, source observations, boundaries and critical path.
- `crates/engine/src/lib.rs`: public bounded document/display types, error/diagnostic contract; forbids unsafe in this crate.
- `crates/engine/src/dom.rs`: html5ever 0.40.1 TreeSink, bounded UTF-8 chunks, flat arena/index links, no recursive ownership or traversal/drop.
- `crates/engine/src/layout.rs`: limited deterministic static text/block flow, HTML whitespace collapse, bold/headings/br, document coordinates.
- `crates/engine/tests/static_document.rs`: nine real integration tests.
- `evidence/verification/engine-*.txt`, `engine-source-hashes.json`: actual check/failure transcripts and source fingerprints.

## Acceptance evidence

| Gate | Actual result | Evidence |
|---|---|---|
| Input at most 262144 UTF-8 bytes, arena at most 32768 nodes; overflow rejected | PASS integration negative test | `engine-green.txt` |
| Malformed HTML tree recovery/entities/Unicode and foster/adoption cases | PASS integration tests through actual html5ever | `engine-green.txt` |
| Head/script/style/template content absent from display; CSS/interactive gaps visible | PASS integration tests; explicit diagnostics | `engine-green.txt` |
| Deep traversal/drop uses flat ownership | PASS 8000 nested div parse/layout/drop | `engine-green.txt`; source review still required |
| Viewport width 64..8192, deterministic bounded run positions | PASS negative/wrapping and 256 generated malformed-token sequences | `engine-green.txt` |
| Rust formatting | `cargo fmt -p opensequoia-engine -- --check` exit 0 | `engine-format.txt` (successful check is silent) |
| Warning-free package Clippy | `cargo clippy -p opensequoia-engine --all-targets --locked -- -D warnings` exit 0 | `engine-clippy.txt` |
| Final package tests | `cargo test -p opensequoia-engine --locked` exit 0; 9 passed, 0 failed/ignored; final integration suite 5.20 seconds | `engine-green.txt` |
| Actual Windows surface/runtime and independent review | Parent/Windows workstream required; not established by library tests | Root S01 acceptance |

Toolchain observed: rustc 1.98.1 `(48a229cea 2026-09-01)`, cargo 1.98.1 `(797e8a9bc 2026-08-05)`; x86_64 Windows MSVC workspace toolchain. Root owns exact Cargo.lock (html5ever 0.40.1 and resolved graph). No candidate conformance suite, browser benchmark, fuzz campaign or dependency security audit was run by this agent.

## Failures and TDD limitation

Tests were written before implementation. The initial semantic stub returned `InvalidTree`, but its first red attempt failed in rustup component setup before tests ran (`engine-red.txt`). It is **not a valid red behavior result**. Root repaired the exact toolchain sequentially; this interrupted TDD cycle is retained honestly. Initial compilation then exposed ambiguous string-cache `AsRef` selection (`engine-initial-compile-failure.txt`), fixed by atom equality/explicit string type. Clippy exposed two collapsible conditionals (`engine-initial-clippy-failure.txt`), fixed without allows or weakened checks. Final checks ran after fixes.

## Limits / security / performance

This engine does not implement CSS semantics, DOM mutation/WebIDL, JS/Wasm, networking, navigation, sandboxing, site/origin isolation, persistence, media, extensions, sync or accessibility nodes. Approximate glyph advances are fixed scalar estimates; GDI/font/shaping metrics may differ and require a shared measured text contract. It is not suitable for remote hostile pages. Parser resource bounds limit input/arena, not proven total memory, CPU duration or side effects of dependency internals. No relative Brave performance result is claimed.

Unsafe-free local source does not imply unsafe-free dependencies: html5ever/markup5ever/tendril and Windows bindings have unsafe internals needing root's locked source review. No C/C++ browser engine is deliberately included in this engine crate; selected graph/artifact evidence belongs to root. Source searches cannot prove absence in a binary.

Tree insertion relies on html5ever's private TreeSink semantics; overflow handles retain unique names/ids without arena allocations until a 256-byte input chunk returns, then parsing rejects the document. Layout guards node/parent identity, traversal steps, run count and height. Independent review should stress adoption agency, templates and invalid internal links.

## Next executable tasks

1. Parent reruns workspace format/Clippy/tests, actual Windows paint/scroll/resize keyboard smoke and independent review before S01 acceptance.
2. Tighten shared platform text metrics (not CSS claim); resolve any review defects with failing regression tests.
3. Establish renderer lifecycle, sandbox and typed bounded IPC before any remote content.
4. Review production TLS provider/certificate policy; RustCrypto provider warning remains a blocker. Evaluate Boa 0.21.1/Wasmi selected graphs and conformance with no native engine fallback.

No commits, manifest edits, Craft session changes or other owners' source edits were made by this agent. Root maintains master checklist and integration records.
