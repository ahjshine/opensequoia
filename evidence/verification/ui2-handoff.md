# S02d native application handoff

Owner: `/root/native_chrome`. Source is frozen for root integration and independent R03 review. Exclusive changes: `crates/app/src/windows.rs`, `crates/app/src/chrome.rs`; evidence `ui2-*`. No manifests, root records, commits or Craft session were edited.

## Implemented contract and behavior

`windows::run(PageSource, bool)` presents the native application; the boolean selects paint smoke. `windows::app_smoke()` runs the bounded native-control application probe. Root `main.rs` declares `chrome` and routes modes. The window consumes `Browser`/`PageSource`/request-generation state and root `RenderController` directly; it contains no HTML parser or file/process waits. Polling is 30ms, repeated resize reflow is debounced 120ms; cached tab frames may be used immediately for the same source. Navigation/reload clears prior content, while reflow retains same-source display until completion.

Native controls include tab buttons, New/Close, Back/Forward/Reload, an Address label and native EDIT, Go and Guide. A fixed 32-button pool avoids growing HWND allocations across navigation. Native labels/disabled states/focus use documented Windows controls; crowded tabs have visible-window selection and Ctrl+Tab, not a completed tabstrip accessibility provider. Native keyboard routes implement Ctrl+L/T/W/Tab/Shift+Tab, Alt+Left/Right, F5, Enter and tab traversal; page click focuses scrolling. Native document text UIA, IME integration, full shaping/selection/high contrast and all production accessibility remain unfinished.

All GDI output consumes the real Rust engine display list. It is clipped below native chrome and above status. Content uses the previously tested fixed-cell Consolas contract; native chrome uses OS Segoe UI. Fonts/controls/DCs are owned, restored/destroyed in order, and state/mapping guards are released before reentrant Windows calls. No Rust pointer is stored in HWND data; WM_COMMAND validates the matching owned control handle, and no arbitrary LPARAM pointer is dereferenced. The initial local-volume preflight remains available to root input loading with its existing race limitation.

## Review corrections

R03 rejected-Go finding: after invalid native text or source validation, the committed address is restored with notice retained. Source/history/old correct frame remain unchanged. The actual native smoke sets a rejected HTTPS address, invokes Go via the owned BUTTON, and compares the native EDIT and current request key before valid navigation. Source returns before controller submission on rejection; request-key evidence alone is not an independent process-spawn count.

R03 delayed viewport finding: reply acceptance now emits a same-source Reflow request when an accepted foreground result has an outdated width. `poll` submits the emitted request outside the state borrow. The deterministic regression creates real engine frames at 960/640 and models tab1 Loading, tab2 resize/completion, selecting Loading tab1, delayed 960 reply, fresh 640 reply and rejected old-generation repeat. It checks reflow generation, unchanged history and eventual frame width. This verifies the transition/correspondence logic, not native input latency or arbitrary scheduler conformance.

Fixture identity oracle: root updated Welcome page copy, invalidating the old unique word `rendering`. The current unchanged criterion is content/source discrimination using the **first** text run `OpenSequoia` for Welcome versus `Browse` for Guide. Fixture and source hashes are in `ui2-source-hashes.json`. The earlier `ui2-app-smoke.log` failure was overwritten by a subsequent successful command; its exact tool-observed result is separately noted without claiming an intact original raw log.

## Commands actually run

Windows x64, Rust 1.98.1, current workspace version0.2.0.

- `cargo test -p opensequoia --locked`: at initial integrated slice, 33 tests passed, zero ignored; later root/peer changes add tests. This old run is not the final whole-workspace verdict.
- `cargo test -p opensequoia --locked delayed_foreground_reply_after_other_tab_resize_emits_current_width_reflow`: one targeted regression passed; 34 filtered at the observed run. `ui2-resize-regression.log`.
- `cargo clippy -p opensequoia --all-targets --locked -- -D warnings`: final exit0. `ui2-clippy.log`.
- `cargo build -p opensequoia --locked`: final exit0. `ui2-build.log`.
- `target/debug/opensequoia.exe --app-smoke`: final exit0, `APP_SMOKE_OK navigation=1 back=1 forward=1 tabs=1 close_reset=1 native_controls=1 child_rendered=1 paints=11`. `ui2-app-smoke.log`.
- `target/debug/opensequoia.exe --smoke-test`: post-fix actual native paint exit0, 105 drawn text runs and child_rendered=1. This run precedes the final identity-oracle-only change; root should reverify final artifact. `ui2-paint-smoke.log`.

The application probe uses real native EDIT set/read and BUTTON BM_CLICK -> owned WM_COMMAND, actual child rendering and expected page content. It does not prove physical keyboard input, screen-reader behavior or UIA roles. No CUA interactions were made by this agent; root owns those checks under current authorization. Paint counts vary with native scheduling and are not frame-performance measurements.

## Remaining acceptance and next action

Root must bind final code/fixture hashes, run full quality/runtime/audit/package gates, independently re-read OS unsafe paths and perform native keyboard/UIA/scroll/resize checks. Craft must inspect the rebuilt0.2.0 artifacts. No remote browsing, AppContainer/site/origin/permission sandbox, signed update, modern CSS/JS/Wasm/API compatibility or full Brave parity is claimed. Local file spelling/volume checks are not production filesystem isolation. Current model/renderer/tab limits are development bounds and do not narrow retained mission scope.
