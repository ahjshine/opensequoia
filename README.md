# OpenSequoia

Windows-first Rust browser project in active development. The application and engine are Rust; documented Windows APIs provide native window/text drawing. No Chromium, Blink, V8, Electron, CEF or WebView2 is embedded.

Current app version: **0.2.0**. Native tabs, Address/Go, Back/Forward/Reload, per-tab history and asynchronous child rendering now work for built-in pages and trusted local HTML. Renderer children use Windows Jobs, a stdio handle allowlist and bounded cancellation/exit checks. **This is not a completed browser and is not ready for hostile remote content.** Remote networking, full CSS (only a bounded static CSS text/box subset with block background fills is present), JavaScript, WebAssembly, AppContainer/origin isolation, profiles, extensions, sync and most retained Brave parity are still missing.

Build with Rust 1.98.1, MSVC Build Tools and Windows SDK (set WindowsSdkDir if outside the standard SDK root):

```powershell
cargo build --release --locked
target/release/opensequoia.exe
target/release/opensequoia.exe --inspect
target/release/opensequoia.exe --inspect fixtures/background.html
target/release/opensequoia.exe --smoke-test
target/release/opensequoia.exe --smoke-test fixtures/background.html
target/release/opensequoia.exe --process-smoke
target/release/opensequoia.exe --app-smoke
```

Default input is `about:welcome`; `about:guide` explains the controls. Press Ctrl+L, enter either built-in address or a local HTML filename, and press Enter. Ctrl+T/W opens/closes a tab, Ctrl+Tab/Shift+Tab switches, Alt+Left/Right navigates history, and F5 reloads. Click the page to scroll with arrows, Page Up/Down or Home/End. The current final-tab policy opens a fresh Welcome tab. That policy and 32-tab/128-history bounds are development states, not full Brave workflow parity.

An optional local UTF-8 development file is limited to 256 KiB; URI/device/share/observed reparse or remote-volume paths are rejected, with explicitly remaining path/volume race limitations. Use only trusted development fixtures; Job membership does not restrict origin, filesystem or network privileges. Text metrics are deliberately limited; advanced shaping, selection and document accessibility remain unimplemented.

`--inspect` uses the parser/layout without a window. `--smoke-test [local.html]` requires actual child-rendered native paint before automatic close. `--app-smoke` tests native controls, page identity, history, tabs and rejected-address restoration. `--process-smoke` verifies actual child valid/malformed/restart/reap behavior. None proves site isolation, AppContainer or authenticated capability IPC. `--renderer-worker` is an internal development protocol mode, never an arbitrary privileged browser service.

```powershell
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
python scripts/package_dev.py
```

The ZIP at `evidence/artifacts/raw/opensequoia-0.2.0-win-x64.zip` is an unsigned development payload, not an installer or production release. It retains its historically recorded MPL-2.0 and conservative transitive license notices; Windows fonts and platform DLLs are OS services, not bundled browser implementation. Clean-machine CRT availability is not yet verified. Current source code license: MIT. Historical development artifacts retain the licenses and notices recorded at their creation. Brave names identify the reference baseline; OpenSequoia is not affiliated with Brave Software. See SPEC.md, CONSTRAINTS.md, ARCHITECTURE.md, FEATURE_PARITY.md, BLOAT_DECISIONS.md, SECURITY.md, BENCHMARKS.md, BASELINE.md and TASKS.md for retained requirements, evidence and precise gaps.
