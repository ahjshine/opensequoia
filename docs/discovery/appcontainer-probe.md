# S03c AppContainer implementation and probe

Status: **local candidate, blocked on the required loopback `WSAEACCES 10013` observation**. This is not production-sandbox acceptance, formal READY, or permission to enable remote browsing. The final oracle has 24 passing assertions and four unmet network-dependent assertions; it has never printed `S03C_PROBE_OK`.

## Source and environment identity

- Lane: `sandbox-probe`, branch `crew/s03c`, base `2027b2274f975f68f583a67aaed16f6b9ec89b38`. The frozen candidate SHA is reported in `opensequoia-crew/reports/s03c.md`.
- Actual tools: `rustc 1.98.1 (48a229cea 2026-09-01)`, `cargo 1.98.1 (797e8a9bc 2026-08-05)`, Windows `10.0.26300.0` x64.
- `windows-sys =0.61.2`; its inspected generated declarations and the existing `Win32_Security_Isolation`/`Win32_Security_Authorization` features are used. No dependency or manifest was changed by this lane.
- Winsock ABI was checked against installed SDK `10.0.26100.0` `um/WinSock2.h:361-392` and `windows-sys-0.61.2/src/Windows/Win32/Networking/WinSock/mod.rs`: `WSADATA` is 408 bytes on x64, `SOCKADDR_IN` is 16 bytes, and `WSAEACCES=10013`. Local Rust declarations have compile-time size assertions. No WinSock Cargo feature was added.
- Final owned-source hashes are in `target/sandbox4/final-source-hashes.json`; acceptance input hashes and command/output hashes are in `evidence/verification/sandbox4-acceptance.json`.

The API implementation follows the already inspected contract in `docs/discovery/appcontainer.md`: [CreateAppContainerProfile](https://learn.microsoft.com/en-us/windows/win32/api/userenv/nf-userenv-createappcontainerprofile), [DeleteAppContainerProfile](https://learn.microsoft.com/en-us/windows/win32/api/userenv/nf-userenv-deleteappcontainerprofile), [SECURITY_CAPABILITIES](https://learn.microsoft.com/en-us/windows/win32/api/winnt/ns-winnt-security_capabilities), [UpdateProcThreadAttribute](https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-updateprocthreadattribute), [GetTokenInformation](https://learn.microsoft.com/en-us/windows/win32/api/securitybaseapi/nf-securitybaseapi-gettokeninformation), [CreateProcessW](https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-createprocessw), and [SetNamedSecurityInfoW](https://learn.microsoft.com/en-us/windows/win32/api/aclapi/nf-aclapi-setnamedsecurityinfow). The implementation uses handle-based `SetSecurityInfo` for the same DACL operation. These links identify API documentation; no external retrieval was performed in this implementation lane.

## Root-integration boundary

The lane base intentionally lacks root-owned `main.rs`, `input.rs`, and `crates/app/Cargo.toml` integration. Those actual files were not edited. Runtime checks used a generated copy wholly under `target/sandbox4/oracle-integration`, with the five root inputs copied read-only from `opensequoia-crew/controller`. Their hashes and the test-only status are recorded in the acceptance JSON and `target/sandbox4/integration-provenance.json`. This fixture is not root-gate evidence and is not committed.

The controller must integrate the lane and rerun the real root CLI/workspace checks. In particular, the mandated root command `cargo test --target-dir target/sandbox4 -p opensequoia --locked` was not represented as run in this unintegrated lane. The controller-owned `input.rs` also needs `cargo fmt`; only its generated test copy was formatted here.

## Observed launch diagnosis and final policy

The inherited implementation failed `CreateProcessW` with `ERROR_ENVVAR_NOT_FOUND 203`. Controlled runs preserved under `target/sandbox4/` showed:

1. Adding `SystemDrive`, ProgramData variables, or a per-drive current-directory entry did not fix 203.
2. Synthetic `LOCALAPPDATA` pointing to the fresh staged directory fixed the launch. On this host, `APPDATA` and `USERPROFILE` were not required. The final environment is only `SystemRoot`, `WINDIR`, and synthetic `LOCALAPPDATA`; the real user environment/profile is not inherited.
3. Canonical `\\?\` fixture paths were rejected by the existing root-owned CLI path validator. The final code retains canonical paths for identity checks but passes a local-drive absolute path to the denial child. This was a path-validation failure, not an `ERROR_ACCESS_DENIED` observation.
4. `TcpStream::connect_timeout`, blocking `connect`, and direct Winsock AF_INET/AF_INET6 attempts did not yield 10013. Final bounded nonblocking `connect` plus `WSAPoll`/`SO_ERROR` returns `SO_ERROR=0` after two seconds. No timeout is relabelled as a denial.

`RendererSandbox::prepare` creates a fresh bounded profile name and owns only an exact successful profile creation. Staging is a fresh generated child of `target/sandbox4/runs`; canonical-parent and reparse checks prevent path escape. `stage/renderer.exe` is separate from `controls/deny.dat`. User and LocalSystem retain full control; only the generated AppContainer SID receives generic read/execute on the run root, stage, and executable. The controls directory, deny fixture, and owner receipt remain user/SYSTEM-only. No Everyone/ALL APPLICATION PACKAGES grant, personal profile, broad capability, firewall change, or loopback exemption is used.

Launch uses zero explicit capabilities, `CREATE_SUSPENDED`, the existing creation-time Job and exactly three inherited stdio handles. Before `ResumeThread`, the exact child token must report AppContainer=1, `EqualSid` with the generated SID, the integrity SID authority/RID for exact Low 4096, and zero capabilities; process identity and owned-Job membership are also required. Any query, identity, path, attribute, or resume failure terminates the owned Job and observes termination with a bounded wait. There is no ordinary fallback.

The development-only `spawn_sandboxed_control` varies only an included sentinel handle or a two-process positive-control Job. Ordinary and production sandbox launches remain one process with three stdio handles and the 256 MiB development Job limit.

## Actual evidence

`evidence/verification/sandbox4-acceptance.py` builds with `--locked --offline`, executes the actual application and Rust process helper, checks exact markers and positive controls, and writes the owned JSON/log. Final result: **24 passed, 4 unmet**.

Passing observations include:

- Real ordinary/staged and AppContainer renderer IPC returns the same expected display. Suspended token evidence is `app=1 sid_equal=1 integrity_rid=4096 capabilities=0`.
- Ordinary denial control reads and rewrites identical synthetic fixture bytes, connects to the fresh IPv4 loopback listener, and sends one synthetic byte. AppContainer read and write both return exact OS error 5; the fixture remains byte-identical and the listener accepts no sandbox connection.
- Wrong executable path fails closed. Closing the owned Job kills the real sandbox child. A two-process descendant control starts and is reaped; the one-process Job returns quota error 1816.
- An included sentinel handle reads the exact 32-byte marker. The otherwise identical excluded-handle child exits with `0xc0000008` (`STATUS_INVALID_HANDLE`); this is recorded as the strict-handle observation, not a graceful `NOT_INHERITED` marker. Ordinary graceful handle-inclusion/exclusion regression tests also pass.
- A pending pipe read returns cancellation error 995. Killed-parent OS handle closure kills the real AppContainer child. Explicit owner-receipt recovery deletes the profile and named staging tree.
- Explicit cleanup markers succeed and the oracle observes zero remaining entries under `target/sandbox4/runs`.

The four unmet assertions are exactly:

1. `sandbox smoke exit` (actual exit 1).
2. `actual helper loopback WSAEACCES 10013` (not observed).
3. `broker exact denial/token observation` (requires 10013).
4. `complete sandbox smoke` (no `SANDBOX_SMOKE_OK`, no `S03C_PROBE_OK`).

The final error is `Winsock WSAPoll loopback connect timed out with SO_ERROR=0`. Earlier AF_INET and AF_INET6 evidence is preserved in `target/sandbox4/denial-winsock-poll.log` and `target/sandbox4/denial-winsock-ipv6.log`; final evidence is in `sandbox4-acceptance.log`.

## Commands actually run

All paths below except the lane source/evidence paths are generated under `target/sandbox4`:

```powershell
rustfmt --edition 2024 crates/app/src/windows_sandbox.rs crates/app/src/windows_process.rs crates/app/src/worker.rs evidence/verification/sandbox4-process-helper.rs
cargo fmt --manifest-path target/sandbox4/oracle-integration/Cargo.toml --all -- --check
cargo test --manifest-path target/sandbox4/oracle-integration/Cargo.toml --target-dir target/sandbox4/test-build -p opensequoia --locked --offline
cargo clippy --manifest-path target/sandbox4/oracle-integration/Cargo.toml --target-dir target/sandbox4/clippy-build -p opensequoia --all-targets --locked --offline -- -D warnings
$env:OPENSEQUOIA_S03C_INTEGRATION_ROOT='C:/Users/deepa/OneDrive/Dokumen/ChatGPT/opensequoia-crew/controller'
python evidence/verification/sandbox4-acceptance.py
```

Results: owned Rust format checks and `git diff --check` passed; app tests **40 passed, 0 failed/ignored**; focused Clippy exited 0 with warnings denied; the acceptance oracle exited 1 with the four explicit unmet assertions above. Logs are `target/sandbox4/cargo-test-final.log`, `target/sandbox4/clippy-final.log`, and the owned acceptance JSON/log.

After root integration, the required real-root rerun is:

```powershell
cargo test --target-dir target/sandbox4 -p opensequoia --locked
cargo clippy -p opensequoia --all-targets --locked -- -D warnings
python evidence/verification/sandbox4-acceptance.py
```

## Observed, derived, and inferred claims

- **Observed:** the runtime markers, process exits, token queries, exact file errors, quota/cancellation/strict-handle statuses, listener results, unchanged fixture check, and zero remaining staging entries described above.
- **Derived:** the ownership/API contracts and ABI layouts come from direct SDK/binding/source inspection and the previously captured Microsoft documentation. Attribute backing, SID, handle, Job, and capability structure lifetimes follow those contracts.
- **Inferred, not established:** the zero-capability AppContainer appears to have its loopback attempt silently dropped on this host. Whether this is external Windows behavior or an implementation defect is unresolved; R04 must assess it. No all-network denial claim follows.

## Remaining production gaps

The required 10013/final-smoke gate and independent R04/root reviews remain open. Real-root integration checks, broader Windows/architecture coverage, live mitigation-policy verification, secure installer/staging race handling, authenticated origin/capability IPC, site isolation, TLS/origin/storage policy, downloads/extensions, and production threat-model acceptance are unmet. Name collision currently fails closed rather than reusing a profile. Path/reparse checks reduce but do not eliminate same-user TOCTOU races. This finite synthetic probe does not establish exploit resistance or remote-browsing safety.
