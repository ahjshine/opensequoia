# D04 — Initial threat model and security acceptance gates

Owner: Windows/security analyst. Date: 2026-11-22. State: **design proposed; no security guarantees or product controls verified.** This model covers the full retained browser mission and identifies gates for incremental implementation. D03 provides actual API/source observations; D02 supplies component findings.

## Threat scope and assets

Assets: profile credentials/passwords/autofill; cookies, tokens, sync keys and history; per-origin storage and permission decisions; bookmark/download/session integrity; device files, clipboard, camera/microphone and OS identity; update trust keys; process availability and CPU/RAM; user interaction and privacy. Private sessions must not leak durable state into normal profiles or logs.

Attackers: malicious sites and cross-origin frames; compromised renderer/extension; active network attacker and hostile TLS endpoints; malformed HTML/CSS/JS/Wasm/images/fonts/media; malicious downloads; corrupted/imported profiles; local low-privilege processes sending messages or accessing IPC; compromised dependency/build/update hosting; abusive resource workloads. Browser scripts are adversarial programs even when every implementation line is Rust.

Assume the OS kernel/drivers and documented APIs are trusted to their published security boundary, with patching responsibilities tracked. A same-user fully privileged local attacker can usually inspect/modify a user's process/profile; do not promise protection beyond OS boundaries. Authenticated UI mediation still needs to stop untrusted renderer requests from becoming broker authority.

## Trust boundaries and abuse cases

| Boundary | STRIDE concern / concrete abuse | Proposed control | Required evidence before completion |
|---|---|---|---|
| Browser broker -> renderer | Renderer escape gains profile/files/process rights | Per-site renderer; AppContainer/LPAC capabilities; immutable startup mitigations; no profile token/handle | Child attempts to read/write profile, launch process, access arbitrary pipe/network/device and receives denial; token/capability/mitigation queries; failure to establish policy aborts launch |
| Renderer -> broker IPC | Forged origin/document/permission, oversized or stale message, pointer confusion | Versioned owned messages; byte/count limits; broker-assigned document/site IDs; allowlisted state transitions and narrow services | Protocol fuzz/property tests; wrong sequence/site/epoch/handle rejected; EOF/crash restart; bounded allocation/backpressure; unauthorized requests denied |
| Browser UI -> content | Page spoofs address/security/permission UI or captures shortcuts | Separate trusted chrome; broker owns displayed URL/security state and permission prompts; user gesture tokens bound to document/expiry | Spoof/focus/fullscreen/activation regression tests and UIA separation |
| Network -> parser/engine | Bad TLS, redirect origin confusion, compressed bombs, attacker script | Rust network/TLS or documented OS crypto interface; checked decompression limits; origin and redirect policy; isolated parsers | Untrusted/expired/wrong-host certificate rejection; HSTS/mixed-content/redirect/cookie/CORS/PNA/WPT tests; bounded malformed body handling |
| Origin/site -> storage | Cross-site cookie/cache/IDB/service-worker leakage; private-to-normal persistence | Explicit profile/private/top-level-site/origin keys; opaque origins distinct; structured permission checks | Cross-origin and partition tests; private close/recovery leaves no persistent records; path/corruption negative tests |
| Fonts/images/media/GPU -> services | Decoder memory corruption, CPU/allocation exhaustion, device attack | Rust decoders or documented OS codecs in restricted services; bounded inputs and IPC; shader/device validation | Candidate/native audit; fuzz corpus and runtime process access denial; oversized resource rejection and recovery |
| Extension -> browser/services | Overbroad API authority, unsigned/tampered package, content-script privilege crossing | Extension principals isolated from sites; declared permissions and consent; signed package/update verification; least-privilege broker APIs | Baseline API matrix; malicious extension fixtures; host/API permission/serialization/signature/update tests |
| Download/import -> filesystem | Traversal, overwrite, device path, symlink/reparse attack, malicious executable | Broker-controlled file destination; canonical path/reparse/ownership checks; quarantine/MOTW and explicit launch | Traversal/UNC/device/reparse/rename race fixtures; no automatic execution; cancel/resume and corruption tests |
| Profile persistence -> state | Corrupt file injects origin/permissions or unbounded resources | Versioned schema; size limits; atomic transactional updates; validated imported records | Crash consistency, truncated/oversized/incompatible data recovery and fuzzing; permissions cannot be granted by untrusted imported schema |
| Credential manager/sync -> secrets | Plaintext/password theft, bad pairing, unauthorized server, key reuse | Windows credential/DPAPI interfaces with documented limits; authenticated encrypted sync, key lifecycle and explicit consent | Round-trip/negative tests without secret logs; sync service availability/license/auth evidence; backup/import/private scope tests |
| Update/dependency -> process | Rollback, forged publisher/channel, dependency build script compromise | Reproducible pinned lockfile; reviewed scripts/native closure; signed metadata + publisher policy; staged atomic update | Known advisory audit; signature/publisher/hash/channel/rollback negative tests; failed update recovery; installed artifact provenance |

These are acceptance requirements, not completed mitigations. The full security program also needs bug-response ownership, private reports, severity/triage, patch/disclosure SLAs, regression cases and upstream tracking.

## Source-verified Windows mechanisms

Inspected windows-sys 0.61.2 source at windows-rs archive commit `32c3144490c016fe496a0aed769bce60987a2e9d` declares:

- `Security/Isolation/mod.rs:1` CreateAppContainerProfile and `:3` DeriveAppContainerSidFromAppContainerName, imported from userenv.dll.
- `Security/mod.rs:68` CreateRestrictedToken and `:545` SECURITY_CAPABILITIES; `:1166` TokenIsAppContainer.
- `System/Threading/mod.rs:69` CreateProcessW, `:212` InitializeProcThreadAttributeList, `:376` UpdateProcThreadAttribute, `:808` HANDLE_LIST, `:813` MITIGATION_POLICY, `:820` SECURITY_CAPABILITIES.
- `System/Threading/mod.rs:319` SetProcessMitigationPolicy; `System/JobObjects/mod.rs:13` SetInformationJobObject and `:421` JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE.

[AppContainer isolation](https://learn.microsoft.com/en-us/windows/win32/secauthz/appcontainer-isolation) describes resource/identity/network isolation and least privilege. That documentation is evidence of an OS capability, not evidence OpenSequoia has used it correctly. LPAC, user/device ACL interaction, desktop access and supported capability combinations require additional API/source and runtime investigation.

[UpdateProcThreadAttribute](https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-updateprocthreadattribute) documents security capabilities, handle allowlists and startup mitigations. With HANDLE_LIST, the selected handles must be inheritable and CreateProcess's bInheritHandles true; blanket inherited handles are forbidden. Mitigation policy overrides cannot change after process start. SECURITY_CAPABILITIES creates an AppContainer. Launch must use an absolute verified binary path, a correctly quoted mutable command line, a sanitized environment, narrowly scoped handles, validated target identity and checked OS errors. Suspend until restrictions/job assignment are complete where required; no fallthrough to an unrestricted child.

Renderer job objects should handle whole-tree termination and validated resource limits. Memory limits protect availability but do not replace measured browser budgets; truncating required workloads would violate parity. Keep renderer site/origin assignment in broker, including navigation, redirects, opener/frame relationships, service workers and extension contexts. Same-site process sharing is an architecture decision requiring authoritative web isolation rules and cross-origin tests; no accidental global mutable document state.

**Critical platform tradeoff:** a win32k-disabled renderer cannot run the GDI window code recommended only for the first local development slice. Production UI/compositing must be separated or use a justified secure presentation path. Do not disable that mitigation for size/speed convenience. DirectWrite parsing downloaded fonts in the trusted UI is also a new privileged parser attack surface; isolate hostile fonts or adopt an audited Rust implementation. OS codecs/DRM do not become safe by being allowed interfaces.

## Rust implementation and unsafe policy

Require safe Rust for parser/layout/DOM/protocol/storage logic, but also checked allocations/limits and error propagation. `#![forbid(unsafe_code)]` is appropriate for pure engine crates; a minimal platform crate may use reviewed unsafe FFI. No hidden C/C++ engine, codec library or crypto provider may enter through dependency defaults without explicit boundary review. D02 observed Servo's SpiderMonkey/default media/font dependencies; unchanged Servo embedding fails the mission boundary.

Every unsafe site must name its invariant, owner, initialization/alignment/length, lifetime/thread/reentrancy and failure cleanup. Independent review must include dependencies' unsafe paths too. Rust does not establish secure IPC, sandbox policy, certificate validation, resource safety, origin correctness, timing-side-channel protection or JavaScript semantics. Panics cannot unwind across a Windows callback and must not convert hostile input into broker termination.

Evolving tests: cargo fmt/clippy/test; locked source/native/license checks; advisory audit; property tests for protocol/origin/path/serialization; fuzz HTML/CSS/JS/Wasm/fonts/images/IPC/storage/import; sanitizer/dynamic analysis where supported; actual AppContainer negative tests. A source search finding no unsafe/native string or import cannot prove absence; include compiled artifact/import/resource provenance and mode limitations from Craft Anything.

## Networking, permissions and credentials

Prefer a broker-authorized Rust network service. rustls crypto providers must be audited: a Rust-facing TLS library can bundle C/assembly crypto by default. A documented Windows cryptographic/certificate interface can comply with the mission, but certificate-chain/hostname/revocation behavior and portability are explicit choices. Do not bypass cert errors or silently substitute an OS HTTP/browser engine. Networking retains HTTP semantics, caching/cookies, HSTS/CSP/CORS, redirects, proxies, auth, client certs, websocket/RTC and service-worker policies according to spec and parity.

A browser may legitimately visit LAN/local websites; generic server SSRF allowlists are not a replacement for the browser's standard address-space/private-network and permission rules. Web content must never gain arbitrary local-file or credential access from a fetch request. Schemes have explicit policy (including opaque data/file/about/blob semantics); origin parsing and storage partitioning need standards tests.

Permissions originate in broker/UI and bind requesting origin, top-level context, profile/private mode, device/API, persisted decision and user gesture. A compromised renderer's claimed origin/activation is not trusted. Camera/microphone/clipboard/geolocation/download/fullscreen/notifications are separate policy gates. Password/autofill filling must bind actual document origin and trusted UI consent; DPAPI/credential storage does not prevent renderer-mediated theft by incorrect fill policy.

## Initial milestone exposure and blockers

The first slice may load only controlled local test fixtures and present a documented HTML/layout subset. An in-process native window has ordinary user privileges, so arbitrary downloaded/local HTML is also potentially hostile. Do not advertise it as secure browsing, open remote URLs, enable script/extensions or process untrusted imported profiles before corresponding security gates. Controlled local input is a development milestone boundary, not an authorized scope reduction; retained capabilities remain in the backlog.

Remote browsing is blocked until all of the following have reproducible evidence: fail-closed restricted child launch; real file/network/device denial tests; authenticated bounded broker IPC and document identity; standards URL/origin/redirect/cookie/permissions rules; TLS negative tests; malformed-input and allocation bounds; crash/recovery behavior; threat review and native/dependency audit. If one gate fails, independent local or conformance work can continue without pretending it passed.

External release blockers: signing certificate/publisher identity, secure update metadata/hosting, vulnerability reporting channel, sync authentication/availability, extension catalog/package policy, DRM licenses and OS codec configurations have not been established. No integration is implemented merely because Brave has it.

## Skills, verification and handoff

Read installed `agent-skills:security-and-hardening` and imported ECC `rust-patterns` (`C:/Users/deepa/.claude/skill-library/rust-patterns/SKILL.md`, metadata origin ECC). Applied threat-model-first/STRIDE, adversarial input boundaries, minimal public contracts, bounded channels and unsafe invariants. Their generic web-app recipes are adapted to browser threat boundaries, and broad claims that compilation implies correctness do not meet this mission's evidence standard. No skill-specific permission restriction is invoked for this authorized discovery/design work.

Actual verification: D03's OS/toolchain, downloaded crate manifests/source/hash and Microsoft documentation inspection; threat matrix cross-checked with D02 engine recommendation. No sandbox/TLS/IPC/credential/fuzz test or security audit tool was run. Acceptance met: named assets/attackers/trust boundaries, source-backed Windows mechanisms, concrete negative gates, unresolved blockers. Unmet: independent review and implemented controls. Parent should integrate this model into SECURITY.md/ARCHITECTURE.md without promoting proposed controls to verified status.

Next actions: implement bounded display-list/local fixture slice; independently review unsafe Windows lifetime code; then define broker-owned document/site identity and protocol, implement a restricted child whose only authorized behavior is IPC, exercise access-denial and crash cleanup tests, and only after those gates add network/TLS with origin and parser limits. Preserve this report as initial model and append changes with evidence.

## Initial implementation delta

S01 now has a bounded Rust local parser/layout and Win32/GDI presentation. S02b now has a one-shot own-executable worker with S02a bounded framed IPC, parent timeout/kill/reap checks, actual healthy/malformed/restart probe and controlled sleeping-child timeout test. These reduce implementation uncertainty but do **not** satisfy the production broker/site/AppContainer/peer-authority model above. Native window rendering still occurs in an ordinary process. Preflight local-file reparse/volume checks retain races and are not a sandbox.

Current worker spawns no descendants. Cleanup evidence covers the direct child only; before allowing child-spawn capability or hostile remote workloads, require job-tree lifetime ownership, handle policy, cancelable I/O and real access-denial tests. I/O-finished polling now prevents an unconditional join after cleanup, returning failure if blocked; failure/detached thread is not claimed as complete cleanup. The finite process probe and safe Rust do not establish secure IPC or isolation. See windows.md/evidence/verification/windows-process-handoff.md for commands/results and pending review.
