# OpenSequoia development checkpoint — 2026-11-22

Status: **project in progress; full browser and release acceptance unmet**. Code checkpoint `eb94cf4` on `codex/foundation`; initial baseline/specification checkpoint `a293bc9`. Source fingerprints bind the tested files independently of documentation/evidence commits. No remote push or release publication.

## What runs and verification

Three Rust crates provide bounded UTF-8 HTML tree construction into a flat arena, a declared limited block/text display list, Windows-native local text painting, bounded version-1 IPC and a one-shot renderer process probe. The native window currently uses the in-process engine; the worker is not yet its production renderer. No embedded browser fallback or bundled native browser-engine implementation was adopted. Windows OS text/window APIs and generated Rust bindings are documented permitted platform interfaces; dependency unsafe remains reviewed only within stated coverage.

Root `scripts/verify.ps1 -SkipWindow`: format, Clippy with warnings denied, **30 tests (11 app, 9 engine, 10 IPC), zero failed/ignored**, release build, real headless parse/layout and actual healthy/malformed/restarted child probe passed. Source hashes matched afterward. Separate actual native paint smoke passed `WINDOW_SMOKE_OK paints=1 drawn_text_runs=113`. Independent R01/R02 source reviews and reruns preserved found/fixed defects and unresolved limits. Tests include real file/junction rejection, child timeout/reaping, finite malformed inputs, node/input/geometry limits and blocked-thread behavior. They are not a standards suite or a completed fuzz campaign.

The observed ASCII overlap defect was reproduced with OS font measurement before correction and passed after fixed-cell Consolas drawing across size/bold/DPI cases. Native resize/reflow was observed before the user stopped Computer Use with physical Escape. Manual scroll/Escape, post-fix screenshot and assistive-technology verification remain open; no further Computer Use occurred.

Cargo-audit 0.23.0 found no known vulnerabilities/informational warnings for the 37-entry lockfile (34 registry packages plus three workspace crates), using RustSec commit `d1fe6ac3eba823a3515e849045f36dc34543885d`, no ignored advisories. This is a database-bounded result, not security proof. Windows CI is configured with pinned action/toolchain/auditor revisions, tests and licensed ZIP packaging; it has not run on a remote CI service.

## Artifacts and performance

Executable: target/release/opensequoia.exe, **908800 bytes**, SHA-256 `930a4415a37137ad33ce77e4c82c134aa1ae0be5dcae14bc8449104ffb7042b8`. Windows metadata identifies OpenSequoia 0.1.0; Authenticode **NotSigned**. Development ZIP: evidence/artifacts/raw/opensequoia-0.1.0-win-x64.zip, **359254 bytes**, SHA-256 `e1f6b22bb9f84113fc3f873e57a682d7b83cceb09936d083c172fc10b68183bf`. Four-file unpacked payload **1250966 bytes** includes executable/MPL license/README/transitive notices. Two identical-input package runs matched hashes. This is not an installer, signed update implementation or complete browser distribution.

Brave baseline is stable 1.100.58 / Chromium 158.0.8127.42; source pins/verified wrapper digest and environment are in BASELINE.md. Seven exploratory hidden warm new-profile idle samples gave median total private commit **235.38 MiB**, working set **456.21 MiB**. Cold startup, page load/frame/interaction, modern-workload and energy measurements remain absent. The subset OpenSequoia payload cannot establish a fair size/memory/speed advantage over a complete browser; no concrete full-browser performance budgets are satisfied.

## Craft evidence and interpretation

Initial MCP target opening was unavailable and passive runtime read was consent-gated. After human approval, supported Deep mode and the existing CLI's documented per-tool commands provided static analysis without reinstalling the connection or bypassing a rejected operation. Durable investigation/report/ledger/handoff are under docs/investigations and evidence/artifacts/brave-opensequoia.cany. Source questions used pinned Git objects; artifact questions used Craft.

The static milestone contains 13 findings with 19 exported evidence records; record validation found no missing finding references. Final CFG evidence `ev_5c812b8afc08c37a8703187d37d55c51dc6ec0d6532cc81a074ac6d89b5ff51a`; final resources `ev_15a6b690669ec0bd4b2b9cb3e76f6b6be92cda0177889ec448ab11d12c37518b`; final ZIP membership `ev_ec8eed8efe42aa4f623e196215e20703eff68bf5c2ac5f9693bf7c9285e3f8b0`. These IDs belong to the durable CLI ledger, not the earlier empty MCP session. Some installer layout evidence could not be reloaded/cited; the affected hypothesis was abandoned and remains a recorded tool limitation, not a proved finding.

Observed artifact facts include OpenSequoia PE resources/imports, corrected CFG flag and ZIP membership; Brave's installed LastChange matches selected core commit, VPN binaries remain packaged, and Installer/chrome.7z is the largest version-directory file. The updater wrapper has its own version 1.3.361.151; payload unpacking/build provenance remain unresolved. Resource/flag observations do not prove runtime security or component absence. Craft code_signature:null is an unreported signature field; OS Authenticode separately establishes signature status.

## Mandatory remaining work and next execution

S01 manual interaction/dependency-unsafe coverage remains under review. S02a/S02b are verified development leaves; the navigation/session umbrella remains pending. Next: Windows process-tree/job ownership, inherited-handle policy, cancelable I/O, AppContainer negative tests and authenticated origin/capability protocol; then asynchronous child-to-window integration and crash recovery. Existing direct-child reaping does not establish descendant/parent-death cleanup, and bounded I/O failure may detach a blocked thread.

TLS/certificate/origin/storage/permissions, CSS/live DOM/JS/Wasm and standards APIs, tabs/history/bookmarks/downloads/profiles/private sessions/passwords/autofill/settings/import-export/Shields/devtools/accessibility/IME/extensions/sync/media/graphics/RTC remain required. Pinned WPT/Test262/Wasm inputs exist, but their adapters and suites have not run. Full modern website compatibility is ambition, never proved by finite tests.

Release additionally requires a trusted signing/update identity, installation/recovery verification, secure intake/response, service eligibility/authentication and codec/DRM licensing. Widevine redistribution rights and external sync/extension integration access are not established. These limitations do not justify deleting retained scope. Full dependency security/unsafe review, fuzzing, hostile-process/IPC tests and baseline performance program remain gates. Resume from TASKS.md, CONSTRAINTS.md and the evidence/handoffs; do not restart discovery or claim this checkpoint complete.
