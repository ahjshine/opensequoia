# S03c AppContainer implementation handoff

Owner: `sandbox-probe`; branch `crew/s03c`; base `2027b2274f975f68f583a67aaed16f6b9ec89b38`. Status: **frozen-source candidate with the required loopback 10013/final-smoke gate unmet**. Not a fully verified S03c completion, formal READY, production sandbox, or remote-browsing enablement. The gate ledger was not edited and manual review is not self-attested.

## Owned files changed

- `crates/app/src/windows_sandbox.rs`: fresh profile/SID and private staging, ACLs, exact token policy, Winsock denial helper, owned cleanup/recovery.
- `crates/app/src/windows_process.rs`: three-attribute suspended AppContainer launch, pre-resume checks, fail-closed controls, existing Job/stdio/cancellation preservation, owned helper paths.
- `crates/app/src/worker.rs`: real renderer equality, bounded ordinary/AppContainer denial probes, exact markers, cleanup observation.
- `evidence/verification/sandbox4-process-helper.rs`: actual Job/quota/handle/cancellation/parent-death controls with value cards.
- `evidence/verification/sandbox4-acceptance.py`: repeatable stdlib build/run/oracle with a value card; prints `S03C_PROBE_OK` only if all assertions pass.
- `evidence/verification/sandbox4-acceptance.json` and `.log`: final actual results.
- `docs/discovery/appcontainer-probe.md`: API/source identity, diagnosis, commands, observed/derived/inferred claims, and residual gaps.
- `evidence/verification/sandbox4-handoff.md`: this handoff.
- `target/sandbox4/**`: generated overlays, binaries, diagnostic logs, provenance, and hashes; not a commit input.

No actual `main.rs`, `input.rs`, Cargo file, root record, `.unlazy` state, engine/IPC file, other lane, legacy checkout, merge, push, or Git remote action was changed/used.

## Root integration required

The lane base lacks root integration. Actual probes used only a generated test fixture under `target/sandbox4/oracle-integration`, copying five root-owned inputs read-only from `opensequoia-crew/controller`: `Cargo.toml`, `Cargo.lock`, `crates/app/Cargo.toml`, `crates/app/src/main.rs`, and `crates/app/src/input.rs`. Their hashes are in `sandbox4-acceptance.json`; generated files are uncommitted. This is not root acceptance evidence.

Root must merge the frozen lane commit into `crew/controller`, format the root-owned `input.rs`, rerun real root/workspace quality gates and `python evidence/verification/sandbox4-acceptance.py`, and leave S03c blocked unless exact 10013/final-smoke evidence appears. The root command `cargo test --target-dir target/sandbox4 -p opensequoia --locked` was not claimed as run in the unintegrated lane.

## Implementation and unsafe invariants

1. Profile names are bounded generated ASCII; only exact successful fresh creation confers deletion ownership. Collision/other-profile reuse is never accepted. The source executable is absolute, canonicalized, non-reparse, regular, and capped at 64 MiB.
2. `target/sandbox4/runs` is canonical-parent/reparse checked. Every run root is freshly created. The staged binary and sibling controls/deny fixture have separate DACLs. User/SYSTEM full control is retained and only the generated SID gets loader/root RX. No inherited/broad application-package grant is used.
3. SID and token buffers use initialized aligned integer backing with 68-byte SID and 64 KiB token caps. Length, subauthority count, pointer-within-buffer, and integrity authority/RID are checked before SID APIs or unaligned C-structure reads. OS SID/ACL outputs use their documented `FreeSid`/`LocalFree` owner exactly once.
4. `SECURITY_CAPABILITIES` has the owned SID, null capability pointer, zero count/reserved. The SID and structure outlive `DeleteProcThreadAttributeList`; HANDLE/JOB arrays and all handle owners outlive creation and attribute deletion. Aligned attribute storage is capped at 64 KiB and deleted only after successful initialization.
5. `CREATE_SUSPENDED` child token AppContainer flag, expected SID equality, exact Low 4096, zero capabilities, process ID, and owned-Job membership are verified before the one owned initial thread resumes. Failures terminate/wait on the owned Job; no ordinary fallback exists.
6. The existing noninherited kill-on-close Job, one-process/256 MiB development limits, HANDLE_LIST stdio policy, and cancelable I/O remain. Only the explicit development control adds a known sentinel or two-process quota control. `ContainedChild` closes process/Job before its final sandbox owner.
7. Winsock uses checked SDK layouts, one successful `WSAStartup` paired with `WSACleanup`, and one valid socket paired with `closesocket`. Address/port bytes, FIONBIO, `WSAPoll`, and `SO_ERROR` use initialized exact-size buffers. A two-second poll is bounded; timeout/`SO_ERROR=0` is an error, never 10013.
8. Cleanup validates the canonical generated root and bounded private owner receipt, then deletes only the named profile/files/directories. No recursive deletion occurs. Reparse points, identity changes, unexpected entries, and cleanup failures are errors. Parent-death recovery runs only after parent/child termination is observed.

## Actual diagnosis and regression repairs

- Original 203 launch failure was fixed by synthetic staged `LOCALAPPDATA`, not by inheriting the user environment or weakening policy. Other tested environment additions did not fix it; `APPDATA`/`USERPROFILE` were unnecessary on this host.
- Verbatim `\\?\` fixture paths caused root CLI path rejection, not file denial. Drive-form child arguments fixed it while canonical identity checks remain.
- The first helper output-path correction created its directory too late. The original full suite showed 38 passes/one failure; after creating the directory before `rustc`, the original parent-death test and full suite passed.
- The negative AppContainer raw-handle probe exits `0xc0000008` even with `GetHandleInformation`. The final oracle requires this exact strict-handle status only in the excluded run, while the included run must read the marker. It does not claim a graceful marker or suppress the crash.
- A helper cancellation refactor initially produced a nested `Option` compile failure in two test consumers; removing the extra `Some` restored all six process tests and then the full suite. No assertions/timeouts were weakened.
- Domain/FFI/lifetime review added staging-parent/reparse checks, denial target confinement, compile-time Winsock sizes, bounded helper I/O joining/reaping, and an integrity-SID regression with a value card.

## Commands and results

Environment: Windows `10.0.26300.0` x64; Rust/Cargo 1.98.1; windows-sys 0.61.2; SDK 10.0.26100.0 declarations.

- `rustfmt --edition 2024 <four owned Rust files>` and subsequent `--check`: pass.
- `cargo fmt --manifest-path target/sandbox4/oracle-integration/Cargo.toml --all -- --check`: pass after formatting the generated controller-owned `input.rs`; real root formatting remains a root action.
- `git diff --check`: pass.
- `cargo test --manifest-path target/sandbox4/oracle-integration/Cargo.toml --target-dir target/sandbox4/test-build -p opensequoia --locked --offline`: **40 passed, 0 failed/ignored**; log `target/sandbox4/cargo-test-final.log`.
- `cargo clippy --manifest-path target/sandbox4/oracle-integration/Cargo.toml --target-dir target/sandbox4/clippy-build -p opensequoia --all-targets --locked --offline -- -D warnings`: exit 0; log `target/sandbox4/clippy-final.log`.
- `$env:OPENSEQUOIA_S03C_INTEGRATION_ROOT='C:/Users/deepa/OneDrive/Dokumen/ChatGPT/opensequoia-crew/controller'; python evidence/verification/sandbox4-acceptance.py`: exit 1; **24 passing assertions, four unmet**. No `S03C_PROBE_OK`.

Final source/evidence hashes are in `target/sandbox4/final-source-hashes.json` and input/output hashes in `sandbox4-acceptance.json`.

## Token/denial/lifetime evidence

Observed passing controls:

- `SANDBOX_RENDER app=1 sid_equal=1 integrity_rid=4096 capabilities=0 equal=1 spawned=2 reaped=2`.
- Ordinary `app=0 read=0 write=0 network=0`, one accepted synthetic byte, unchanged fixture.
- AppContainer `SANDBOX_PROBE_FILES app=1 read=5 write=5`; `SANDBOX_DENIAL_OBSERVATION accepted=0 fixture_unchanged=1`.
- Wrong path denied; Job close kills; two-process control starts/reaps; one-process descendant error 1816; included marker read; excluded strict-handle `0xc0000008`; cancellation 995; killed parent causes child death.
- `SANDBOX_CLEANUP profile_deleted=1 staging_removed=1`, `SANDBOX_LIFETIME_OK reaped=1 cleanup=1`, and `SANDBOX_PARENT_DEATH killed=1 child_reaped=1 profile_deleted=1 staging_removed=1`; oracle `remaining=0`.

Unmet, exactly: `sandbox smoke exit`, `actual helper loopback WSAEACCES 10013`, `broker exact denial/token observation`, and `complete sandbox smoke`. Actual final network error: `Winsock WSAPoll loopback connect timed out with SO_ERROR=0`. AF_INET, AF_INET6, blocking/nonblocking evidence is retained under `target/sandbox4/`; no accepted sandbox connection was observed, but that alone is not the required 10013 proof.

## Four-pass result and next actions

Implementation, domain-expert reread, correctness/security/FFI/lifetime defect hunt, and low-cost polish were performed; the final improvement pass left only documented formatting/root integration actions and the explicit network blocker. No further loopback iterations are planned in this lane.

R04 must independently assess SID/ACL/token/attribute/handle/Job/cleanup safety, the strict-handle observation, and whether loopback timeout is an implementation defect or host behavior. Root integration and independent runtime review remain mandatory. Production origin/capability IPC, site isolation, TLS/storage, mitigations, secure installer/staging races, other Windows targets, and all remote-browsing gates remain open.
