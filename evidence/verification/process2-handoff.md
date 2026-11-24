# S03a Windows renderer containment handoff

Owner: `/root/process_containment`. Date: 2026-11-22. Status: implementation/self-verification complete for the local renderer process leaf; root independent review/integration required. Not an AppContainer, privilege, origin or production browser security completion.

## Owned files and exact interface

- `crates/app/src/windows_process.rs`: documented OS process/Job/pipe attribute boundary; `ContainedChild`, `spawn(&Command)`, bounded `try_wait`, Job-wide `kill`, owned Job membership/process identity query, and `cancel_io(&JoinHandle<()>)`.
- `crates/app/src/worker.rs`: keeps `forbid(unsafe_code)`; `render(html: &str, width: u32) -> Result<DisplayList, String>` uses only the actual child. HTML is capped before cloning; valid framed Display must match requested width and successful process exit. No in-process fallback.
- `evidence/verification/process2-*`: source docs/receipts, test-only helper source, actual logs and current fingerprints.

Root added `mod windows_process` and the windows-sys Security, Threading, Pipes, IO and JobObjects feature groups. No manifest/lockfile/root record/commit was changed by this leaf.

## Implementation and unsafe invariants

Rust 1.98.1's installed Windows process docs mark custom ProcThreadAttributeList/spawn_with_attributes as nightly. The stable build therefore uses documented `CreateProcessW` and `STARTUPINFOEXW`, not guessed Rust extensions or an embedded native browser. Microsoft docs were fetched and captured in `process2-official-docs.json`; selected source URLs include:

- https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-updateprocthreadattribute
- https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-createprocessw
- https://learn.microsoft.com/en-us/windows/win32/api/ioapiset/nf-ioapiset-cancelsynchronousio
- https://learn.microsoft.com/en-us/windows/win32/procthread/job-objects

The fresh unnamed Job is noninheritable, has kill-on-last-close, one active process and a 256 MiB private-memory development limit. `JOB_LIST` assigns it during process creation, before child code runs. `HANDLE_LIST` passes only the three child stdio ends. Parent ends are noninheritable; no parent Job/process/thread handles are included. Native program path is absolute and supplied separately from escaped argument data; no PATH lookup or shell evaluation occurs in the launcher. Only SystemRoot/WINDIR loader context is copied into the child's environment. This is not an audited DLL-search or sandbox policy.

The attribute storage is aligned initialized `Vec<u128>` with a bounded sizing query. Both attribute value arrays own stable heap backing, outlive DeleteProcThreadAttributeList, and retain all handle owners through CreateProcessW. Delete is called only after successful initialization. Fresh handles transfer to OwnedHandle/File exactly once; successful process creation's initial thread handle is promptly closed. The process ID and own-Job membership are checked before returning. Native buffers are initialized, bounded and correctly sized; command strings/environment are terminated UTF-16. All raw Win32 calls and safety comments remain in the platform module.

Supervisor timeout terminates the whole owned Job, observes child reaping and invokes CancelSynchronousIo on the exact owned I/O thread while waiting boundedly. Cancellation is a request, not completion: the guard checks thread finish before joining. ERROR_NOT_FOUND means no pending I/O, not proof finished. An uncompleted/error path still returns explicitly and may detach the thread; it is never counted as complete cleanup. AppContainer and a compromised renderer exporting handles/asking external brokers to act remain outside this leaf's guarantees.

## Actual checks (Windows 11 x64, Rust 1.98.1, windows-sys 0.61.2)

Commands were run from the project root:

```powershell
cargo test -p opensequoia --locked windows_process::tests -- --nocapture
cargo test -p opensequoia --locked worker::tests -- --nocapture
cargo run -p opensequoia --locked -- --process-smoke
cargo clippy -p opensequoia --all-targets --locked -- -D warnings
```

Platform **6 passed, 0 failed/ignored**; worker **5 passed, 0 failed/ignored**; actual CLI probe **healthy/malformed/restart passed, 3 spawned and 3 reaped**; scoped all-target Clippy **exit 0**. Current logs: `process2-platform-tests.log`, `process2-worker-tests.log`, `process2-smoke.log`, `process2-clippy.log`. These are filtered leaf tests, not a full current-workspace verdict.

The six OS/platform probes prove:

1. Membership is present immediately after spawn; closing the last Job handle terminates the still-live real child.
2. An otherwise identical two-process Job control really starts/kills/reaps a harmless PowerShell child; production one-process limit rejects the same direct CreateProcess workload with quota error **1816**. This corrected R03's finding that a generic exception catch could falsely count an unrelated failure as containment.
3. A real pending pipe read completes with cancellation error **995** while its writer remains open, so EOF cannot substitute for cancellation.
4. Native argument escaping preserves quotes/trailing slashes and rejects NUL/oversized text.
5. A known inheritable sentinel is readable with an explicitly included positive control and excluded from the production handle list.
6. Killing a real helper parent without Rust destructors terminates its actual child through the parent's noninherited Job.

The parent-death/handle helper at `process2-helper.rs` includes the actual production Windows module; it is not a mocked Job. Tests compile it to `target/process2-probe-helper.exe` using `rustc --edition=2024 <helper> --extern windows_sys=<current target deps rlib> -L dependency=<target deps> -o <helper exe>`. The selected compatible Cargo-built Windows rlib is captured by the test's target/deps search. Helper build warnings for unused standalone-module APIs are recorded separately; it is development tooling and is never in the four-file app package. Main application Clippy remains warning-free. No personal data, arbitrary executable or browsing profile is used.

The request-width regression first encodes/decodes a valid 640-wide wire response (shape validation succeeds), then verifies the 320-wide request boundary rejects it, 640 accepts it, and an unsuccessful child exit rejects it. Peer/origin authentication is still missing.

Initial missing-module and concurrent UI compile failures were not semantic RED tests and are not represented as such. A scoped lint failure found an owned unused-mut (fixed) and an unrelated UI nested-if (reported to its owner). The leaf introduced real negative/positive controls and regression checks; no ignored tests or warning allowances were added to browser source.

## Limits and precise next actions

Only the available Windows 11 x64 environment was executed. JOB_LIST is documented for Windows 10+, but other Windows versions/architectures remain untested. Job limits do not establish least privilege, filesystem/network denial, site/origin isolation, IPC authentication, browser compatibility or performance benefits. The memory limit is a development guard, not a measured full-browser budget. No target PE/resource analysis was performed by this source/runtime leaf; root's Craft workflow owns final artifacts.

Root must independently review the native invariants and run the current full workspace/runtime gates after UI integration, then bind final source fingerprints (`process2-source-hashes.json`) and compiled artifact identity. Continue with AppContainer/restricted capability policy, origin/session/capability IPC, DLL/module provenance and negative escape tests before remote content. Preserve the explicit cancellation-failure path and active-process quota control rather than claiming universal cleanup or sandboxing.
