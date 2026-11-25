# OpenSequoia architecture and decisions

Status: proposed full architecture plus the verified 0.2.0 local application increment. Implemented evidence is bounded by the accepted development milestone. See docs/discovery/rust-engine.md, windows.md and threat-model.md.

## ADR-001: Rust engine boundary

Adopt useful Rust components after source/license/transitive dependency review; build missing browser integration. html5ever is a parser, not a browser. Servo unchanged is incompatible with the requested boundary (native JavaScript and other dependencies). No Chromium-family fallback. Consequence: standards JavaScript, layout/rendering, media and browser APIs require substantial implementation/integration work. A full replacement for a mature engine is a multi-subsystem program; partial conformance stays explicit.

## ADR-002: first local slice and interfaces

Separate a safe `opensequoia-engine` library from `opensequoia` Windows application. Engine owns bounded arena DOM, HTML tree construction and limited block/text layout; display-list values cross the platform drawing boundary. App owns event loop, local input, window sizing/scroll offset and Windows drawing. Use generated windows-sys bindings for documented OS services. GDI text/raster output is an allowed Windows service; it does not supply HTML/CSS/DOM or script interpretation. It is an initial renderer with known text/shaping limitations; final GPU/compositing and accessibility integration remain necessary.

Implemented S02a adds `opensequoia-ipc`, a safe Rust, explicit version-2 bounded frame codec (tagged text and block-fill records) with strict payload/geometry/UTF-8 validation. The historical S02b one-shot local worker probe was integrated into the native application in 0.2.0: its background controller reads trusted local input, launches a child renderer, and publishes validated display lists. The native UI no longer parses or lays out documents in process. Authenticated origin/capabilities and production sandbox remain next steps. No extra third-party runtime was added for IPC or navigation. Refer to docs/reviews/ipc-lifecycle.md, application-wave2.md and evidence/verification for tested boundaries.

The first slice has no network, JavaScript, profiles or external integrations. This is a development gate preceding hostile remote content, not a permanent removal of requirements. Parser work still treats inputs as malformed and bounded. Avoid unsupported markup silently looking like full compatibility.

## ADR-003: production process design (not yet implemented)

Privileged browser broker owns trusted UI and capabilities. Site-isolated low privilege renderer processes own DOM/script/layout. Separate bounded network/storage/media utility services where least privilege requires them. IPC uses versioned typed messages, length/resource bounds, origin/capability IDs and validation on receipt. OS handles and storage/network permissions are broker issued. Renderers do not select arbitrary files or endpoints. Resource/crash recovery preserves process isolation. Renderer lifecycle is a prerequisite for remote browsing, not a size optimization to remove.

Windows process sandbox requires AppContainer/restricted tokens, job limits, mitigations and handle inheritance policy tested on supported Windows targets. Threat model governs every exception. Certificate trust through an evaluated Rust TLS provider or documented Windows TLS interface must preserve validation; Rust crates with default bundled native crypto providers require explicit review.

## ADR-004: dependencies and external integrations

One locked dependency graph; direct versions fixed. Archive/checksum source inspection plus release SBOM/Craft analysis track Rust and native payloads. OS DLL import is distinct from bundled native implementation. Codec/DRM/password APIs may be documented OS services; unavailable licensed codecs/Widevine or Brave sync/extension backend are never represented as implemented. Component roadmap must account for Rust JS/Wasm conformance, font shaping, GPU services, multimedia and extension semantics.

## ADR-005: evidence and performance

Pin actual Brave stable source and shipped artifact, preserving architecture/service distinctions. Runtime benchmarks use clean temporary profiles and whole process trees with equivalent workloads. Static executable imports/strings cannot prove runtime security, speed or absence. Craft evidence complements source, conformance, runtime and profiling. Benchmark improvements of a subset prototype are never reported as equivalent full-browser gains.

## ADR-006: Windows control-flow guard

Craft's first S02 executable snapshot reported NX/PIE/high-entropy VA flags but no CFG. Enable the compiler's documented `-C control-flow-guard=yes` for Windows MSVC targets in `.cargo/config.toml`, then inspect a rebuilt artifact. This strengthens an available platform mitigation; its header/table evidence is not proof of sandboxing or exploit immunity. Signing and full process policy remain unmet. Compare before/after digests and flag evidence without weakening functionality or other security controls.

## ADR-007: local application navigation and renderer ownership

Implemented and independently accepted for local development in 0.2.0. `browser.rs` owns tab IDs, per-tab history and monotonic request generations. `controller.rs` owns a single background work loop, bounded per-tab coalescing and cached HTML for resize reflow. `chrome.rs` and `windows.rs` own native controls and paint; only accepted display-list values cross from renderer to UI. Navigation clears previous-source frames; late replies and wrong-width renderer responses cannot acquire current authority. The selected-tab late-width regression triggers same-source reflow without adding history.

Consequences: navigation and reflow avoid blocking the UI on parse/layout or child waits. A running renderer call is not immediately canceled when a tab closes; results lose authority, and lifecycle timeouts/Job cleanup bound the tested exchange paths. File/volume operations and aggregate cache/display memory need further cancellation and measurement. Development bounds are 32 tabs, 128 history entries per tab, 4096-byte addresses and 256 KiB HTML per cached page. They do not replace full browser requirements or establish memory superiority.

## ADR-008: creation-time Windows Job and stdio policy

Implemented 0.2.0 on the tested Windows 11 x64 host. `windows_process.rs` uses documented `PROC_THREAD_ATTRIBUTE_JOB_LIST` to assign the Job before execution and `HANDLE_LIST` for exactly three stdio handles. The Job is noninheritable, kills children on last close, and limits its development renderer to one process and 256 MiB private memory. Pipe cancellation waits for observable completion; child exit is checked and reaped. Six real platform probes include positive controls for descendant and handle-inheritance denial. Source and unsafe invariants were independently reviewed in R03.

Consequences: direct-child-only ownership limitations of S02b are superseded for the verified paths. Jobs do not remove ambient user privileges, prevent file/network access, authenticate origins or establish a sandbox. AppContainer/restricted-token policy and actual denial tests are mandatory before remote content. Supported API claims come from Microsoft documentation; only the current host was exercised.

## ADR-009: release GUI subsystem

Release Windows builds use `windows_subsystem = "windows"`; debug builds retain console diagnostics. Internal workers still receive explicit stdio pipe handles. CLI development modes are tested through redirected handles in the verification script. This avoids an extra console window when users launch the release application. The correction changes executable bytes despite identical length; the console draft is historical and must never substitute for the final GUI package. Craft must confirm PE subsystem 2 and package correspondence; release signing/install/update remain unmet.
