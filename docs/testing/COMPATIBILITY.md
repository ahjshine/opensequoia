# Compatibility program v1

Program start: 2026-11-22. The retained scope is unchanged. Suite pins in `suites.json` identify the next reproducible test inputs; a pin is not a test run. Clone each suite at its commit, record harness/browser revisions, configuration, Windows build/architecture and resource settings. Keep upstream manifests and all required failures visible.

| Suite / workload | Purpose | Adapter and required evidence | Current state |
|---|---|---|---|
| Web Platform Tests | HTML/CSS/DOM/layout, networking/storage/permissions/workers/media/graphics/accessibility | actual app WebDriver/testharness or reftest runner; results with upstream IDs, pass/fail/timeout/crash/skip/unsupported and regressions | adapter not implemented; no WPT pass claimed |
| Test262 | ECMAScript semantics and host integration distinctions | pinned Rust JS engine harness; flags/features/negative cases preserved; unsupported separated | engine integration not implemented |
| WebAssembly spec tests | validation, interpreter/runtime semantics and proposals | pinned Rust Wasm integration runner and host bindings; per-proposal results | not implemented |
| Differential local corpus | Brave versus OpenSequoia parsing/layout and interaction | clean temporary profiles, fixture hash, viewport/DPI/font/OS identical; standards decide discrepancies | S01 unit/runtime fixture subset only |
| Modern site/app corpus | navigation, auth, forms/uploads/downloads, storage/offline/service workers, canvas/WebGL/WebGPU/RTC/media | versioned public fixtures or reproducible app snapshots; scripts/consent/redaction; no personal accounts | corpus/harness pending; remote content blocked before sandbox |
| Extensions | manifest versions, APIs, permissions, content scripts, isolation/update | required API inventory from baseline; deterministic fixtures and compatibility report | extension runtime missing |
| Accessibility | native UIA semantics, keyboard, focus, text selection/assistive tech and page semantics | actual application UI Automation plus screen reader/keyboard procedures | S01 window/scroll only; full provider missing |

Never convert failing required tests into skips to improve totals. A passing parser test proves that tested behavior only; deterministic estimated glyph widths do not establish CSS text shaping. Unknown/unsupported features remain required work. For each report preserve command, revision, logs, case totals, known failures, environment, standards source, comparison baseline and limitations.
