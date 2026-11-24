# R02 independent IPC and lifecycle review

Reviewer: brave_baseline agent, independent from IPC/Windows authors. Date: 2026-11-22. Status: **S02a codec and S02b controlled local process probe accepted at this milestone. Full browser/security/release unapproved.** This also closes R01's incomplete-workspace format/test confirmation, using the complete three-crate graph.

Reviewed source fingerprints: `evidence/verification/review2-source-hashes.json`. Coverage includes IPC/lib/codec/tests, worker/main/input and changed Windows font drawing, compiler flags, root resources/build script, README, development packaging and Windows workflow. Source review used the installed gstack/native workflows described in R01, in documented degraded mode. No missing helper/audit receipt, external-model review or remote CI execution is claimed. Artifact inspection is owned by the Craft investigation, not R02.

## Findings resolved during review

| ID | Finding and scope | Correction / remaining limitation |
|---|---|---|
| R02-F1 | Original worker supervisor joined its I/O thread unconditionally after reaping only the direct child. A duplicated/inherited pipe held elsewhere could leave that thread blocked and turn cleanup into an unbounded supervisor wait. Confidence 9/10 from source; no failing current one-shot worker descendant case was reproduced because it never spawns descendants. | `worker.rs:20-34` now polls thread completion with a one-second deadline before join. Failure returns explicitly and detaches the unfinished thread; it does not claim cancellation or complete cleanup. A controlled blocked-thread regression releases/joins its fixtures even on regression. Actual pipe cancellation, descendant/parent-death cleanup, job ownership and handle allowlists remain production gates. |
| R02-F2 | Windows CI initially uploaded the bare executable without the license/attribution files collected by the local development packager. Development distribution is still subject to license obligations. Confidence 10/10 from workflow/source. | Workflow now runs `python scripts/package_dev.py` and uploads the ZIP containing executable/LICENSE/README/THIRD_PARTY_NOTICES. Remote workflow was not run by this review; packaging execution and final artifact correspondence remain root evidence. |

No additional concrete current codec integer/slice/allocation, message-type completeness, direct-child lifecycle or error-output defect was found after the corrections. This null result is bounded by the inspected paths and tests, not a guarantee against malicious content or future integration changes.

## Codec and trust boundary

Version-1 wire integers are explicit little endian, not Rust memory layout. Header validates magic, version, type, reserved fields and type-specific payload bounds before payload allocation/read (`codec.rs:160-209`). Whole-frame decode rejects truncation and trailing bytes; stream read intentionally consumes one frame and leaves later frames unread. Invalid frames are documented connection-fatal. Partial writes are documented fatal, and flush/deadline/cancellation belongs to caller.

Inspected limits: four MiB maximum frame including header, 256 KiB HTML, 65,536 text records, one MiB total display text, 1,024 error-message bytes; viewport 64..8192, display height up to one million, positive bounded run geometry and font size 1..256. String/slice consumption uses checked addition and bounds; count multiplication follows the 65,536 cap, so it fits supported 32/64-bit usize. UTF-8 is validated before allocation into String. Record bool and error enum values fail closed. Encode and decode enforce corresponding semantic limits. A frame-size cap is not total-process-memory accounting; payload, record and string allocations can coexist.

The codec has no authentication, request/origin/site IDs, capability checks, replay/session state, OS handle transfer or deadlines. It is a local development transport contract, not production broker IPC. The process smoke compares the returned Display against actual independent same-engine fixture layout, but that fixture comparison does not become authentication for arbitrary browser work. A separate renderer process alone does not establish site isolation or sandboxing.

## Supervisor and application

Own executable is launched with a fixed internal worker mode, piped stdin/stdout, null stderr, CREATE_NO_WINDOW, executable-directory cwd, and cleared environment except SystemRoot/WINDIR. No request selects a process or filesystem path. Worker parses/layouts one bounded request and returns Display or generic Parse/Layout/Protocol errors without echoing input into logs. Internal CLI modes reject extra filenames. Ordinary native-window rendering still uses the local engine directly; child display/native-window integration is explicitly missing.

Concurrent I/O avoids blocking the supervising thread on an unresponsive pipe. Exchange has five-second receive deadline, bounded child-exit polling, kill/reap and I/O-completion handling. Direct child handles/exit status are tracked; success requires returned message and process exit. Last-resort Drop cleanup is not credited as verified reap. `--process-smoke` verifies healthy child, malformed-request rejection and a new child after failure; each direct child is reaped. It does not test parent death, descendants or a compromised renderer retaining arbitrary handles. Environment clearing reduces inherited data; it does not remove ambient Windows user privileges.

After root's visual collision discovery, changed Windows drawing requests fixed ASCII font advances and verifies actual OS ASCII text extent before painting. The real GDI metric test covers 32 size/DPI/bold combinations. Reviewed font handle/buffer and restoration/error paths. Complex Unicode shaping/bidi/grapheme/fallback and pixel-level visual parity remain unsupported; a GDI metric regression is not an accessibility or modern CSS pass.

## Compiler, packaging and CI

`.cargo/config.toml` passes `-C control-flow-guard=yes` on Windows MSVC targets; reviewed conditional syntax and explicit compiler option. Current checks consumed this configuration. Actual PE guard metadata belongs to Craft evidence, not this source read. CFG is a mitigation, not sandbox/origin/TLS/permission enforcement. The configuration does not weaken tests or safe-Rust boundaries.

Build.rs uses structured Windows SDK resource-compiler arguments and packages OpenSequoia/asInvoker/prerelease metadata. Non-Rust build tooling is permitted; no browser engine is supplied by resource compilation. Package script uses locked Cargo metadata, collects conservative registry license texts and emits an explicitly unsigned development ZIP and hashes. Reviewed source only; no local package rewrite by R02. The workflow pins action commits, exact Rust 1.98.1, locked build/test commands and read-only repository permission. It also configures pinned cargo-audit 0.23.0 installation and lockfile advisory checking; R02 did not execute that separately installed auditor or remote job. Interactive UI/assistive technology and production security gates are explicitly outside hosted-CI evidence. Full installed Rust/Cargo/native dependency review from R01 remains applicable; IPC adds a workspace crate and reuses the existing engine dependency, not another runtime or hidden native engine.

## Verification actually executed

Environment: Windows 11 x64, Rust/Cargo 1.98.1, current MSVC/SDK configuration and compiler CFG setting. Logs use `evidence/verification/review2-*`.

| Command | Observed result | Evidence |
|---|---|---|
| `cargo fmt --all -- --check` | exit 0 | review2-format.txt |
| `cargo test --workspace --locked` | exit 0; **11 app + 9 engine + 10 IPC = 30 passed**, 0 failed/ignored | review2-tests.txt |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | exit 0 | review2-clippy.txt |
| `cargo run --locked -p opensequoia -- --process-smoke` | exit 0; actual healthy/malformed/restart direct children **spawned 3, reaped 3** | review2-process-smoke.txt |
| `cargo run --locked -p opensequoia -- --inspect` | exit 0; actual bundled fixture nodes 46, width 960, height 528, text runs 143; one explicit unsupported-feature diagnostic | review2-inspect.txt |
| `cargo tree --workspace --locked --target x86_64-pc-windows-msvc -e normal,build` | completed; workspace IPC reuses engine graph | review2-dependency-tree.txt |

An earlier focused test invocation used an incorrect filter and ran **zero tests** (11 filtered out), recorded in review2-io-deadline-test.txt. It earns no test credit. The final full workspace run actually executed the correctly named unfinished-thread regression. Source hashes were refreshed after final author fixes and include the fixture/compiler/workflow inputs. Recompile waits were not security/runtime failures; no unexecuted check was reported as passed.

Meaningful codec tests include every truncated prefix for selected frames, invalid headers before any payload read, type/geometry/text limits, exact maxima, short reads/writes, fragmented/multiple frames and 1,024 generated mutations with canonical round-trip checks. These finite generated tests are not a fuzz campaign or full input-domain proof. Actual Windows process timeout/kill/reap and local reparse tests complement pure codec cases. The new controlled-thread test proves bounded helper return only, not cancellation of arbitrary OS I/O. Runtime timings in these regression tests check timeout behavior; they are not performance benchmarks.

No release build/artifact internals, sandbox/token/Job tests, malicious extension/download/origin/network checks, WPT/Test262/Wasm, full accessibility, performance comparison or remote workflow result was independently verified by R02. Existing broader gaps remain mandatory, not waived.

## Handoff

Root may integrate S02a and S02b as verified development leaves while keeping the S02 navigation/session umbrella and full browser mission pending. Next executable work: production process/job/handle and cancelable-I/O design; versioned origin/capability protocol; actual child-renderer/native-window integration and lifecycle failure corpus; sandbox/TLS/origin gates before remote input. Preserve primitive limits, connection-fatal behavior, redaction and exact compiler/dependency pins. Re-review if source fingerprints change; final release/ZIP must correspond to the actual reviewed source and Craft artifact evidence.
