# Performance program and evidence

No lightest/fastest/secure-by-Rust claim. Brave must be measured before setting production OpenSequoia budgets. A text-only increment has incomparable functionality to a complete browser; any prototype measurements are labelled separately.

## Reference and environment

Standard Brave v1.100.58 / Chromium 158.0.8127.42, Windows x64. Source/artifact pins: BASELINE.md. Selection 2026-11-22 Asia/Calcutta. Hardware/toolchain details and file hashes are captured in `evidence/environment/baseline.json`; installed-directory size is a snapshot, not necessarily a fresh-install footprint. Report file system allocation/compression and installer choice separately.

## Required workloads and statistics

| Metric | Procedure | Budget / status |
|---|---|---|
| Compressed distribution | download pinned standard standalone Windows x64 artifact, verify published SHA-256 and bytes; OpenSequoia ZIP/signed installer reported separately | measure first; no equivalent budget yet |
| Installed footprint | sum versioned payload and shared files from a fresh test install; preserve packaged caches/DRM/backend differences | installed snapshot only until fresh install tested |
| Cold/warm startup | cold = controlled reboot/cache state; warm = repeated launch, same saved state; time to first meaningful paint/interactive, actual native event evidence | cold not measured; process spawn/idle is not first paint |
| Process-tree memory | clean temporary profile; sum private commit and working set for every descendant at idle, active, background, multi-tab; sample peak, growth and after tab close | baseline idle harness first; do not compare single process against full tree |
| Navigation/page load | versioned local and reproducible network corpus; navigation milestones, load/interactive; same retained settings | blocked until network/modern engine exists |
| Interaction/frame pacing | fixed input/scroll/animation/tab-switch traces; compositor present timing, p50/p95/p99 latency, missed frames | instrumentation/harness pending |
| JS/Wasm/layout/render throughput | pinned workloads and full conformance context | JS/Wasm not integrated |
| CPU/energy | all process CPU deltas, idle and load; energy instrumentation availability explicit | CPU can be sampled; energy not measured |

Use at least seven independent runs for initial runtime samples, medians/range and raw values; rigorous release decisions need predeclared noise thresholds and confidence intervals. Record power plan/background processes/viewport/DPI/fonts, test fixture digest, argument list, run ordering, and measurement limitations. Do not log personal command lines or browsing data. Baseline harness uses an isolated temporary profile and kills only its own verified descendant tree. First measurements of a warm launch followed by five seconds idle cannot establish cold startup, input latency, peak navigation memory or full-load memory.

Optimize only measured bottlenecks. Keep authorized feature-exclusion savings separate from implementation improvements. No budget is waived by this initial measurement state.

## Initial measured milestone

Seven warm, new-profile Brave local-fixture launches, sampled at 250 ms for five seconds, produced median final total private commit **246816768 bytes (235.38 MiB)**, range 216.29..255.96 MiB, and working set **478375936 bytes (456.21 MiB)**, range 419.46..477.45 MiB. Raw samples and limitations: evidence/benchmarks/brave-idle.json. Hidden window, uncontrolled background/power/cache conditions, startup background networking enabled; no cold startup/first paint/frame or comprehensive workload verdict.

The verified OpenSequoia **0.2.0 GUI** development executable is **946176 bytes**, four-file payload **1289128 bytes**, deterministic ZIP **379375 bytes**. Source checkpoint `83451aa` matches all 27 recorded GUI verification inputs; development source has since advanced independently. This local navigation/text-rendering increment lacks most retained scope, so these figures establish artifact footprint only and **do not establish an advantage over equivalent Brave functionality**. Two identical-input packages had SHA256 `743c6594a6e72f0dffdcfbf07537b4900659da35d018933eef39df21d871ff05`. Artifact configuration/hashes: evidence/artifacts/opensequoia-0.2.0-build.json; Craft fnd_18..21 verify GUI metadata/flags/imports/four-member package with loadable evidence. Historical 0.1 and console-draft measurements are not current GUI identity. Runtime whole-browser metrics and concrete comparative budgets remain unmet.

## Exploratory OpenSequoia GUI idle snapshots (M02)

Independent candidate `87adc7d4dd70f798abff264620ff9fd1d6283bcd`, integrated locally as `92082eda2a7b243381229d359aa2105100802b1f`, reconciled the immutable 0.2 GUI measurement without rerunning it. Seven runs contain 20 samples each (140 samples) at 250 ms; raw `evidence/benchmarks/app2-idle.json` SHA-256 is `ac5e2799ab5a8c91a895cccaeb81a2c083a5f69a101a49a8580fb5b2a16c004d`. Final private commit median/range is **1421312 / 1409024–1421312 bytes** (median 1.35546875 MiB), and final working-set median/range is **8282112 / 8273920–8310784 bytes** (median 7.8984375 MiB). Sampled-peak distributions coincide with final values. Every sample contains one extant process; median/min/max final process count is 1/1/1.

Two focused tests and 197 independent acceptance assertions passed, including source hashes, arithmetic, environment fields, wrong-hash rejection and launch-owned PID creation-time cleanup; 12 corruption controls were rejected for their intended assertions. See `evidence/verification/measure2-independent-review.md` and `measure2-acceptance.py`. The immutable EXE SHA-256 is `d80778d7dbaa7f3a520b33e88ab3cb8346cce1c8c31749cba19653ede0511f3e`.

These are **root-only hidden-window exploratory observations, not true process-tree peaks or a Brave comparison**. No renderer child was observed at 250 ms, so renderer resource use, exited-child CPU, true peaks and cleanup of unobserved children remain unknown. Paint/page readiness, startup/navigation/interaction/frame metrics, cold startup, energy and active/multitab/recovery behavior were not measured. Viewport, DPI, fonts and background-process inventory are absent; background workload/cache were uncontrolled. Working-set sums may double-count shared pages. A future new workload needs explicit readiness, high-resolution renderer-lifetime tracking and complete environment capture before any comparative budget or advantage claim.

Craft found the Brave version-directory snapshot's largest file was Installer/chrome.7z (569442764 bytes), followed by chrome.dll (338606160 bytes). The archive can support installation/recovery and is not excluded solely for size. The observed 1146695025-byte version directory excludes other shared/updater/profile allocation and is not a fresh install measurement. See docs/investigations for evidence IDs and package/provenance limits.
