# Pinned Brave baseline

Selection: 2026-11-22 Asia/Calcutta. Standard Brave stable v1.100.58, released 2026-11-18, Chromium 158.0.8127.42. Sources and feature paths: docs/discovery/brave.md.

| Source | Immutable commit |
|---|---|
| brave/brave-browser release repository | `9a51201f8a71f3f2bd377322aaa09c9742588cdd` |
| brave/brave-core implementation customizations | `bd6d26d1bced03c17a98baa86d2cd3425d543d1b` |
| Chromium src 158.0.8127.42 | `52bdd21723d5db0c9ab11716d38feac74828cf84` |

Do not assume other Brave organization repositories are dependencies. Core source defines conditional DEPS and Chromium patches; browser repository provides release/issues/build linkage. Baseline V8, Skia, BoringSSL, ANGLE and FFmpeg pins are in the discovery report. Full Chromium transitive graph and official hermetic build inputs/GN/PGO flags are not yet verified.

## Shipped Windows artifact

[Standard Windows standalone installer](https://github.com/brave/brave-browser/releases/download/v1.100.58/BraveBrowserStandaloneSetup.exe): locally downloaded, **173087584 bytes**, SHA-256 `2802553c89831effe624bd1b0dd8e39c463e03a1224d557441350a5586e559ce`, matching GitHub release digest. Windows Authenticode status Valid, signer Brave Software, Inc. Download verification: `python scripts/fetch_baseline.py`, evidence/artifacts/brave-download.json. Installer was not executed. PE architecture/payload and official build configuration need Craft analysis; x64 naming convention is not itself binary architecture proof.

Installed executable independently observed at `C:\Program Files\BraveSoftware\Brave-Browser\Application\brave.exe`: Windows file/product version 158.1.100.58; 4624976 bytes; SHA-256 `eb7bdd6794feef543c70ed73db5a408636b9f0b86f6977609e8034b1f6da932f`; valid Brave signature. Version-directory snapshot sums **1146695025 bytes**, excluding other shared/updater/profile files and allocation/compression differences. This is not a fresh installation footprint and does not yet prove byte correspondence to downloaded installer payload.

Release metadata lists Windows 10+ and x64, 32-bit and ARM64 standalone artifacts. Full support policy/minimum CPU and real execution on Windows 10/ARM64/x86 remain unverified. OpenSequoia first slice was targeted at the available Windows 11 x64 environment only; planned support includes Windows 10/11 x64 and ARM64 after required platform verification. Do not claim those targets tested.

## Test environment and configuration

Windows 11 Home Single Language 10.0.26200 x64; Intel Core i5-1235U (10 cores/12 logical), 16716918784 bytes RAM. Rust 1.98.1, commit 48a229ceaefd4985c50990b14116b6d856af0985, LLVM 22.1.8, MSVC target. Windows SDK 10.0.26100.0 / VS 18 BuildTools discovered. Full evidence: evidence/environment/baseline.json and docs/discovery/windows.md. Benchmark profiles are newly created temporary profiles, never personal profiles. No controlled reboot/cache/power/background-load configuration is established yet.

OpenSequoia release configuration: opt-level=s, thin LTO, one codegen unit, debuginfo stripped, panic abort; Cargo.lock pins registry packages. This is a recorded initial build choice, not an unmeasured optimization claim. Artifact digest and native/unsafe graph records are added after actual build.

## Craft limitation

Initial connected Craft Anything 0.6.1 was in Clean room mode and client tools omitted target opening/toolset discovery. User approved analysis and the supported Deep setting was enabled. The existing connection (mcp --all-tools) was not reinstalled; the installed CLI's documented per-tool commands provided the analysis route, with a durable `evidence/artifacts/brave-opensequoia.cany` project. Every actual command schema was discovered through CLI help. See docs/investigations for evidence IDs, findings and unknowns. Passive MCP runtime read was consent-gated; manual Computer Use was stopped with physical Escape and is not substituted for Craft evidence.

Craft PE resources in installed brave.exe and chrome.dll embed browser version 158.1.100.58 and LastChange `bd6d26d1bced03c17a98baa86d2cd3425d543d1b`, establishing observed source-identification metadata. Installed chrome.dll SHA-256 is recorded in the investigation. The downloaded installer is an updater wrapper version **1.3.361.151**, not a PE directly versioned 1.100.58; its large B/102 resource contents still need validated unpacking before asserting complete payload-byte correspondence. Official GN/PGO/build provenance remains unknown.
