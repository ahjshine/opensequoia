# OpenSequoia specification

Status: implementation in progress. No production browser, Brave parity, comprehensive compatibility or performance advantage is claimed.

## Mission and immutable scope

Build a Windows-first browser, application **and browser engine implemented in Rust**, retaining Brave's essential workflows, security/privacy and supporting infrastructure. No Chromium, Blink, V8, Electron, CEF, WebView2 or another embedded engine. No native C/C++ browser implementation hidden behind Rust. Documented Windows OS APIs, drivers and services may be called; build/development tools may use other languages. Safe Rust by default, independently reviewed unsafe invariants at necessary OS boundaries.

Retained scope: tabs/windows/navigation/history/bookmarks/downloads; sessions/profiles/private browsing; settings/search/import/export; passwords/autofill/permissions; Shields/tracker protection; accessibility/keyboard/developer tools; required extensions/API compatibility; sync and other retained features identified in baseline inspection. Engine scope includes modern HTML/CSS/DOM/layout/rendering/JavaScript/WebAssembly/network/storage/media, workers/service workers, graphics/RTC and permission-controlled APIs. 100% modern website compatibility is the ambition; finite testing cannot prove it.

Optional services/monetization/promotions may be excluded only on verified source/artifact evidence and policy rationale. Candidate wallet/Web3, Rewards/ads/sponsors, promotions/shopping/referrals, AI assistant, VPN subscription and unnecessary telemetry exclusions never authorize deleting shared retained infrastructure. See FEATURE_PARITY.md and BLOAT_DECISIONS.md.

## Baseline selection

Selection date: 2026-11-22, Asia/Calcutta. Reference is standard Brave stable v1.100.58, published 2026-11-18, Chromium 158.0.8127.42. Brave Origin is a separate artifact variant, not silently substituted. Exact source/artifact provenance belongs in BASELINE.md and docs/discovery/brave.md. Installed directory name alone does not establish authenticity. Baseline source is a behavioral reference, not a plan to translate all upstream code mechanically.

## First executable increment S01

Before coding: accept bounded local UTF-8 HTML (maximum 256 KiB), use actual Rust HTML tree construction, emit a documented limited display list and draw it in a Windows-native OpenSequoia window. Provide deterministic headless inspection of that same engine path. The trusted sample is bundled; externally supplied local files are opt-in development input. No network, script execution, extensions or profile access is enabled before isolation and origin gates.

Acceptance: entity/Unicode handling and HTML recovery tests; bounded input/nodes and stack-safe traversal/drop; real rendering and resize/scroll checks; command-line invalid-input rejection; no fallback engine or production unsafe in engine; Rust formatting/lint/test/build; native OS boundary independently reviewed; executable version/metadata and dependencies recorded. Supported tags/styles and limitations must be visible. Unsupported features remain in backlog, never count as passes.

## Local application increment 0.2.0

Acceptance was recorded in TASKS.md before the navigation/native/process implementation: independent tab history, Back/Forward/Reload, native Address/Go and tab controls, monotonic request identity, asynchronous child-rendered content, rejected-address restoration, stale/width reply rejection, and bounded process/pipe lifecycle tests. The implementation supports `about:welcome`, `about:guide` and trusted local UTF-8 HTML. R03 independently accepted this development milestone with 54 passing tests and actual native application smoke. The GUI subsystem packaging correction has separate artifact/review gates. No modern CSS/JavaScript/Wasm, remote browsing, persistence or full accessibility parity follows from this acceptance.

## Completion criteria

Completion requires every retained parity row and full Rust boundary audit, release security gates, versioned compatibility results, signed install/update/recovery verification, and measured performance budgets established after Brave measurements. A local renderer or browser shell never meets this definition. TASKS.md holds dependency-ordered acceptance criteria, evidence and blockers; ARCHITECTURE.md records decisions and their consequences.
