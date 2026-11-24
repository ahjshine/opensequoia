# D03 / D04 / S01 Windows handoff

Owner: windows_security. Date: 2026-11-22. No commits made; root owns integration.

Files: docs/discovery/windows.md; docs/discovery/threat-model.md; crates/app/src/main.rs, input.rs, windows.rs; evidence/windows/verification.json; evidence/verification/windows-tests.log, windows-clippy.log, windows-inspect.log, windows-smoke.log. Ignored source archives/docs under evidence/windows support the source investigation.

Source identities and environment: Windows 11 build 26200 x86_64, Rust 1.98.1 MSVC; windows-sys 0.61.2/windows-link 0.2.1 and AccessKit 0.35.1 archive commits/checksums in windows.md and verification JSON. D03/D04 findings are observed API/source capabilities plus proposed architecture, not product security guarantees. Rust patterns/testing guidance was read from the installed ECC-origin skills; native threat-model skill applied.

S01 actual behavior: a Rust-engine local document feeds a Win32/GDI native text window. Inspection prints counts/diagnostic categories without document contents. CLI local input is bounded and explicit; direct URI/share/device/unknown-volume paths and observed reparse points rejected. Preflight ancestor/drive-remap races remain and are documented; no sandbox guarantee. Owned thread-local UI state; no raw Rust HWND state pointers or hidden browser runtime.

Verification logs: app6 tests passed0failed0ignored; scoped Clippy -D warnings passed; default --inspect nodes46 width960 contentheight528 text_runs143 diagnostics1; actual debug smoke paints1,113 TextOut runs, exit0. Real local junction test creates only owned temporary local data and proves target intact. Root must independently rerun final integrated workspace/build/UI/artifact checks. Native smoke does not prove layout conformance, safety/performance or accessibility.

Known unresolved work: unsafe review; root UI resize/scroll/Escape observations; final release/Craft import/resource checks; Windows10/ARM64; full dependency/advisory closure; UIA/IME; process/site isolation/AppContainer/IPC before remote content; TLS/origin/storage/privacy controls; JS/Wasm/media/extensions/sync; signed packaging/update. GDI local presentation cannot be moved unchanged into a win32k-disabled renderer; keep compositor separation requirement visible.

Next executable action: root performs integrated format/lint/test/build and actual native window interactions, reviewer completes source and artifact-boundary review, then accept only the bounded local milestone. Advance bounded broker identity/IPC/restricted-child negative tests before adding remote navigation. Do not narrow retained parity or infer full compatibility from the local UI.
