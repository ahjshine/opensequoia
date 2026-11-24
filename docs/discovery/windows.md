# D03 — Windows application and platform investigation

Investigation date: 2026-11-22. Owner: Windows/security analyst. Initial D03 sections are source/platform discovery. S01 implementation and actual verification are appended below; production sandbox, accessibility provider, installer and comparable browser runtime performance remain unimplemented/unverified.

## Evidence and environment

**Observed locally:** `rustc -vV` reports Rust 1.98.1, commit `48a229ceaefd4985c50990b14116b6d856af0985`, LLVM 22.1.8, host `x86_64-pc-windows-msvc`. `rustup show active-toolchain` reports `stable-x86_64-pc-windows-msvc`; installed targets are x86_64 Windows MSVC and wasm32-unknown-unknown. Windows is Windows 11 Home Single Language, version 10.0.26200, build 26200, 64-bit. `vswhere -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath` finds Visual Studio 18 BuildTools. Windows SDK library directories 10.0.19041.0 and 10.0.26100.0 exist. Compiler/linker success still requires a real Cargo build; presence of tooling does not establish it.

Official Rust [Windows MSVC platform requirements](https://doc.rust-lang.org/rustc/platform-support/windows-msvc.html) retrieved during discovery state Windows 10+ / Server 2016+ and Visual Studio 2017 minimum. These are upstream toolchain requirements, not OpenSequoia support claims. Proposed product matrix: Windows 11 x86_64 first; Windows 10 x86_64 remains a required evaluation against the selected Brave baseline and current OS support policy. ARM64 needs a native build/test machine and relevant Brave artifact; no ARM64 target is installed locally. Do not infer arm64 support from Rust target availability.

Raw source snapshots and document captures are under ignored `evidence/windows/`. `evidence/windows/verification.json` records their checksums and the local environment. Registry cache directories initially contained readmes only; source conclusions below use downloaded/extracted crates, not those incomplete caches.

| Component inspected | Source identity | Archive SHA-256 | License / observed implementation |
|---|---|---|---|
| windows-sys 0.61.2 | windows-rs commit `32c3144490c016fe496a0aed769bce60987a2e9d`, `crates/libs/sys` | `ae137229bcbd6cdf0f7b80a31df61766145077ddf49416a728b02cb3921ff3fc` | MIT OR Apache-2.0; generated Rust OS API declarations; `build=false`, empty default features, only windows-link 0.2.1 dependency |
| windows-link 0.2.1 | windows-rs commit `d468916ac27a36fb8a12bafc1bf5c0ec2fe92238`, `crates/libs/link` | `f0805222e57f7521d6a62e36fa9163bc891acd422f971defe97d64e70d0a4fe5` | MIT OR Apache-2.0; `no_std` Rust macros; Windows `raw-dylib` import linkage; no dependencies/build script |
| accesskit_windows 0.35.1 | AccessKit commit `ce8164ba92995cfa86005b6259115e08c8244253`, `adapters/windows` | `ce63f35d6bdcf59f26b76b3379063f738e6412cef46999cc772d46aa3de35adb` | MIT OR Apache-2.0; Rust UI Automation adapter, `build=false`; depends on accesskit/consumer/hashbrown/static_assertions/windows/windows-core; transitive closure not fully audited |

Archives were downloaded from `https://static.crates.io/crates/{name}/{name}-{version}.crate`, extracted with `tar -xf`, and hashed with `Get-FileHash -Algorithm SHA256`. Commit identity comes from each archive's `.cargo_vcs_info.json`; this is published archive metadata, not an independently authenticated Git clone assertion. A production dependency requires Cargo's locked archive checksum, license inventory and advisory audit too.

## First executable platform choice

**Recommendation, not implemented:** use windows-sys 0.61.2 with only Foundation, LibraryLoader, WindowsAndMessaging, Graphics.Gdi and later HiDpi feature groups. Implement one narrowly scoped platform module wrapping Win32 lifetime ownership. The Rust engine produces an owned, bounded display list of rectangles and text runs; the OS presents it. No browser-engine DLL, WebView, Chromium, JavaScript runtime or C++ rendering library is needed for a local HTML subset.

Inspected windows-sys source references (relative to its extracted crate):

| Interface | Actual declaration | Implication |
|---|---|---|
| CreateWindowExW | `src/Windows/Win32/UI/WindowsAndMessaging/mod.rs:71`, imported from user32.dll | Window creation through documented Windows interface |
| DefWindowProcW / GetMessageW | same module, lines 79 / 186 | Native message dispatch; GetMessage returns -1 on error, 0 on quit, positive on message |
| SetWindowLongPtrW | same module, line 418 | Possible opaque state pointer; needs explicit lifetime/reentrancy invariants |
| BeginPaint / EndPaint | `Graphics/Gdi/mod.rs:12` / `:91`, user32.dll | Paint transaction must be paired, including failures |
| SetProcessDpiAwarenessContext | `UI/HiDpi/mod.rs:30` | Per-monitor DPI handling must be configured before window creation |

[BeginPaint documentation](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-beginpaint) says to call only for WM_PAINT, pair each call with EndPaint, and notes output is physical pixels. Therefore tests must cover WM_PAINT failure cleanup, resize, zero-size/minimized windows, DPI changes and coordinate conversion. GDI drawing demonstrates pixels for a subset; it does not establish CSS, full Unicode shaping, smooth scrolling or browser compatibility.

OS-native GDI/DirectWrite/Direct2D/user32 and Windows fonts are allowed platform services through documented interfaces. They are non-Rust Windows implementation, explicitly distinguished from bundled non-Rust browser-engine code. windows-link's Rust import declarations are not a hidden native browser engine. OS DLL imports and bundled files still require artifact verification with Craft Anything after every build. Compiler/runtime import libraries are build linkage, not bundled browser functionality; inspect actual runtime redistributables rather than guessing from a crate name.

For later text, [DirectWrite](https://learn.microsoft.com/en-us/windows/win32/directwrite/direct-write-portal) documents OpenType, Unicode layout, measuring, drawing and hit-testing. Prefer the OS-shipped DirectWrite interface rather than bundling DWriteCore/Windows App SDK. The latter has a different redistribution footprint and must not enter indirectly. windows-sys does not supply all ergonomic COM interface wrappers needed for this work; evaluate the `windows` crate's full dependency closure before adopting it. Font shaping, bidi, selection geometry and CSS metrics need differential tests regardless of the OS API chosen.

## Unsafe interface contract

No unsafe code was written by D03. Before integrating any platform implementation, an independent reviewer must verify:

1. Every FFI call's initialized/aligned pointer and buffer length; UTF-16 storage survives the call, strings needing termination have exactly usable termination and no accidental truncation.
2. Window class/module/function pointers remain valid for every callback; HWND ownership and thread affinity are explicit. UI state must not be referenced after WM_NCDESTROY.
3. Only documented messages permitting valid pointer parameters are dereferenced. Arbitrary messages, wParam/lParam values and posted messages from other local processes are untrusted; they cannot supply an arbitrary Rust object pointer.
4. Callback code contains no unwinding across `extern "system"`. FFI error values are checked correctly. No borrow is held across a call that may reenter the window procedure.
5. GDI object selection/restoration and DeleteObject/DeleteDC/ReleaseDC ownership are paired; stock objects are not destroyed. BeginPaint always pairs with EndPaint. Font/HDC lifetimes and clipping are correct.
6. Coordinates, counts, text sizes and display-list allocation are checked before integer narrowing or OS calls; oversized or nonfinite inputs are rejected. Redraw, zero viewport and scrolling bounds are tested.
7. Global mutable state is avoided. If a raw boxed pointer is used for HWND state, install/retrieve/drop ordering has a single owner and reentrant callbacks cannot create aliased mutable references.

`#[deny(unsafe_op_in_unsafe_fn)]` and focused safety comments are required. Safe Rust elsewhere remains subject to limits, logic security review, panic/DoS tests and conformance testing.

## UI and accessibility work that remains

Browser chrome needs real tabstrip/address editing/history/bookmark/download/settings controls, focus order, keyboard commands, visible focus, high contrast, zoom, touch and screen readers. Native EDIT/control APIs may establish an accessible address field earlier than a custom-painted imitation, but cross-process page accessibility remains a distinct implementation.

Source-verified AccessKit candidate: `adapter.rs:523` documents `handle_wm_getobject`; it defers the UIA call outside held locks because UIA may reenter WM_GETOBJECT. `adapter.rs:624` calls OS `UiaReturnRawElementProvider`; `subclass.rs:153` says subclass adapter creation must occur before first show/focus on the window owner thread, with action callbacks potentially on another thread. Its text modules implement UIA text patterns. These are useful implementation patterns, not proof OpenSequoia is accessible. The adapter includes unsafe Win32/COM operations and panic/unwrap paths; its entire dependency closure and failure behavior must be reviewed before integration.

Microsoft [UI Automation providers overview](https://learn.microsoft.com/en-us/windows/win32/winauto/uiauto-providersoverview) says the provider defines the element tree, control/content views and fragment navigation. Acceptance needs actual UIA tree queries, role/name/state/focus/action checks, browser chrome and document text navigation, Narrator interaction, high contrast, keyboard-only operation and DPI tests. A visible native window is not sufficient.

[Text Services Framework](https://learn.microsoft.com/en-us/windows/win32/tsf/text-services-framework) is a documented OS service for multilingual composition and text input. WM_CHAR-only editing misses IME/composition, dead keys, grapheme navigation and accessibility. Retain these requirements; TSF/IME, clipboard formats, selection and editing are pending.

## Production process and packaging direction

Recommended processes: trusted browser UI/broker owns profiles/permissions/navigation; sandboxed site renderers own DOM/CSS/layout/JS; sandboxed network service makes broker-authorized requests; separate font/media/compositor services isolate hostile fonts, codecs and GPU input. Select services incrementally with measurement, while retaining process/site isolation. An engine process with win32k calls disabled cannot directly present a GDI window: the local GDI subset is not the production isolation design. Do not solve that incompatibility by weakening renderer mitigations.

Use AppContainer/LPAC where supported and proven, explicit capability policy, startup mitigation policies, inherited-handle allowlists and Job Objects. Low integrity alone and a separate process alone are insufficient. See D04 threat model for required negative tests before remote browsing.

Initial local build may be a portable executable solely for development. Product installation/update/recovery still needs per-user locations, profile separation, file associations and default-browser OS workflow, uninstaller/data-retention behavior, version metadata/icon/manifest, reproducible release configuration, signatures and rollback recovery. [WinVerifyTrust](https://learn.microsoft.com/en-us/windows/win32/api/wintrust/nf-wintrust-winverifytrust) returns success only when exactly zero; HRESULT `SUCCEEDED` is incorrect. Authenticode alone does not authenticate OpenSequoia update publisher/version/channel or prevent rollback; signed metadata and pinned publisher/key policy are separate gates. Signing certificate, update hosting and publisher identity are not available or implemented in this task.

## Verification and handoff

Commands actually run: TASKS/AGENTS reads; attempted SPEC/CONSTRAINTS/ARCHITECTURE reads (not yet present during initial discovery); rustc/rustup; CIM OS query; vswhere; SDK directory inspection; crate/doc downloads/extraction/hashes; Cargo manifests, `.cargo_vcs_info.json`, windows-link source, targeted `rg` searches and AccessKit adapter/subclass source reads. No application build, UI automation, sandbox or packaging test was run. Two guessed documentation URLs returned 404; corrected URLs above were retrieved. One PowerShell search using shell brace expansion failed; rerun with explicit path arguments succeeded. Do not cite failed attempts as verification.

Acceptance met: real Windows API/source boundaries, local platform environment, a concrete minimal native window approach and specific unsafe/accessibility/process gates. Unmet: full dependency advisory/closure audit; actual window build; UIA/IME runtime tests; Windows 10/ARM64 tests; secure sandbox/IPC; packaging/signing/update. Next executable action: root pins windows-sys/windows-link and a bounded display-list contract, implement a local trusted fixture window, build with locked dependencies, independently review unsafe lifetime code, exercise paint/resize/keyboard/close and inspect resulting imports/resources via Craft Anything. Then implement sandbox isolation before any remote-document feature.

## S01 implementation and self-verification update

Implemented in `crates/app/src/{main,input,windows}.rs`: bundled trusted HTML via include_str; opt-in bounded UTF-8 regular file loading; mutually exclusive CLI inspection/window/smoke modes; same Rust Document::parse/layout path; native Win32 window and GDI TextOutW presentation of owned display-list text runs; resize/DPI scaling, viewport clipping, keyboard/wheel scrolling and Escape; actual-paint smoke mode with timeout. Engine is implemented separately by the engine analyst. No tab navigation, browser networking, JS/Wasm, profiles, UIA/IME, permissions or production isolation is claimed.

Unsafe FFI remains exclusively in `windows.rs`. Thread-local UI state contains no HWND raw Rust pointer; OS calls run after borrow release, and paint snapshots use owned Rc display values. Debug callbacks catch panic before the Windows ABI; release panic=abort is not a security argument. GDI font selection/restoration/deletion and paint/window/class teardown failures return failure; OS-created cursor/stock brush remain OS-owned. A BeginPaint transaction has explicit checked EndPaint plus emergency Drop cleanup. Independent review and actual UI interaction are root/reviewer responsibilities.

The local file path review identified known directory junctions and mapped drive paths. Implementation now checks the drive type before metadata, rejects nonlocal/unknown volumes and known reparse/symlink ancestors, uses final-component OPEN_REPARSE_POINT, and validates the resulting handle metadata/bounded stream. **These preflights still have directory-ancestor and drive-remap TOCTOU races and are not a sandbox or guaranteed network isolation.** Default bundled input opens no file. Production hostile-content isolation remains blocked and required.

Actual commands/results after integration with root's metadata build:

| Command | Observed result | Log |
|---|---|---|
| `cargo test -p opensequoia --locked` | 6 passed, 0 failed, 0 ignored; UTF-8/boundary/device/options/DPI plus a real local directory junction rejected and its target kept intact | `evidence/verification/windows-tests.log` |
| `cargo clippy -p opensequoia --all-targets --locked -- -D warnings` | exit 0, no warnings | `evidence/verification/windows-clippy.log` |
| `cargo run -p opensequoia --locked -- --inspect` | nodes 46, width 960, content height 528, 143 text runs, 1 unsupported-feature diagnostic | `evidence/verification/windows-inspect.log` |
| `cargo run -p opensequoia --locked -- --smoke-test` | exit 0, `WINDOW_SMOKE_OK paints=1 drawn_text_runs=113`; real debug executable painted before closing | `evidence/verification/windows-smoke.log` |

Scoped rustfmt ran on owned sources. Initial Windows directory-input test failed because opening a directory produced access denial before regular-file classification; added preflight regular-file rejection and kept the original assertion. Root's first metadata build failed RC1015 missing winver.h; root fixed SDK include paths and subsequent test/inspect/smoke builds succeeded. No failed check was counted as a pass. The native smoke precedes final test-only and reserved-name source refinements; root must rebuild/reverify the final integrated artifact before acceptance.

Unmet S01 review/runtime gates: root's independent whole-workspace checks, real resize/scroll/Escape CUA observations, release artifact import/resource review, and focused unsafe review. Production UIA/IME, sandbox/site isolation, standards fonts/metrics and signed packaging remain required future work. No benchmark comparison is made: this local subset cannot establish gains against Brave's retained functionality.

## S01 visual correction and S02b lifecycle update

Root's CUA inspection observed native window creation and resize/reflow but found ASCII words colliding because the static engine's estimated fixed advances differed from Segoe UI glyph widths. PageDown/close verification was stopped by the user's physical Escape and is not a pass; no CUA was run after that stop. A regression using actual OS font metrics reproduced 245px glyph extent against a 207px allocation (size16, DPI96). The same test passed after switching the development renderer to OS Consolas with requested fixed cell width matching the engine's `font_size*3/5` advance, across sizes16/18/24/32, normal/bold and DPI96/120/144/192. Runtime ASCII extent checks reject unexpected font mapping that exceeds allocation. This fixes the observed static ASCII collision; CSS font metrics, Unicode fallback/shaping/bidi/graphemes/selection remain required gaps. Logs: windows-font-red.log and windows-font-green.log. Actual post-fix debug paint smoke passed paints1/drawn_text_runs113; no post-fix screenshot or scrolling/close interaction is claimed.

S02b implemented a separate **local one-shot child lifecycle probe**, not integration of native UI with a sandboxed renderer. `--renderer-worker` reads one bounded S02a Render frame, uses the real Rust parse/layout path, replies with Display or fatal Error, flushes and exits. `--process-smoke` launches three own-executable children and validates healthy rendering, malformed-magic rejection/nonzero exit and a fresh-child recovery; each response matches expected real layout and each direct child is reaped. Pipes/protocol output carry no profile/network data; process-smoke uses only the bundled fixture. Worker environment is cleared except OS loader context. Supervisor I/O runs concurrently, bounded response/exit wait is5s, kill/reap wait1s and completed-thread polling1s. Explicit failure is returned if cleanup cannot be established; emergency Drop cleanup is not counted as success.

An independent review caveat is retained: current own worker spawns no descendants. Direct-child reaping does not prove whole-process-tree termination. Descendants/duplicated pipe handles could keep I/O open; a bounded I/O-finished check now fails instead of blindly joining, but job-tree ownership, cancelable I/O, child-spawn denial, inherited-handle policy, sandbox, authenticated peer/site/capability identity and native window integration remain production gates. No hostile browsing or general secure IPC claim is made.

Latest self-verification: app 11 tests passed, 0 failed, 0 ignored after root added Windows Control Flow Guard compilation, including real local junction rejection, an actual controlled sleeping child timeout/kill/reap and the review-requested unfinished-I/O helper branch. The final helper regression uses a bounded test receiver and releases/joins controlled fixtures even if the implementation regresses; its focused re-run passed (windows-worker-join-test.log). This is a helper test, not pipe cancellation proof. Scoped Clippy -D warnings passed. Actual process smoke previously returned `PROCESS_SMOKE_OK healthy=1 malformed_rejected=1 restarted=1 spawned=3 reaped=3`. Logs: evidence/verification/windows-worker-tests.log, windows-worker-clippy.log and windows-process-smoke.log. Native/process smoke logs predate the CFG rebuild and final test-only refinement; root independently re-verifies final integration/runtime/artifact provenance.
