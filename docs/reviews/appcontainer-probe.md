# R04 independent AppContainer unsafe/security review

Date: 2026-11-24 (Asia/Calcutta). Reviewer: R04 `appcontainer-review`, independent of the S03c implementation and root integration. Scope: frozen integrated snapshot `b3e3ccc0d3f413747ca7c3be76f34761895c8094`, its S03c implementation, root CLI integration, real helper/oracle, and directly affected callers.

**Exact verdict: BLOCKED. S03c is not accepted. The required loopback `WSAEACCES 10013` observation and final sandbox smoke remain unmet. No merge, formal READY, production-sandbox, or remote-browsing claim is supported.**

R04_VERDICT=BLOCKED
R04_CRITICAL=1
R04_INFORMATIONAL=1
R04_REQUIRED_UNMET=4
R04_ACCEPTANCE_PASSED=24
R04_NEW_IMPLEMENTATION_DEFECTS=0
R04_SNAPSHOT=b3e3ccc0d3f413747ca7c3be76f34761895c8094
## Method and degraded workflow

Applied the installed native `review-agent` workflow and the imported gstack security/review, adversarial, QA-only, and test-audit instructions directly. This was the authorized read-only review lane and the `CREW.md` plan exemption applies. Canonical gstack helpers and receipts are unavailable; no session protocol, formal review receipt, outside-model review, telemetry, READY, or SHIP result is claimed. The brief prohibited delegation, so R04 performed the adversarial reread itself rather than dispatching another reviewer.

Read the repository contract, crew rules, discovery/probe records, handoff, acceptance JSON/log, complete scoped diff/current source, helper/oracle, callers, installed windows-sys declarations, and SDK Winsock layout. `docs/discovery/appcontainer.md` is absent from this snapshot and was not found under `opensequoia-crew`; this is finding R04-F2 below. No other checkout's unhashed design document was substituted.

No product source, product test, dependency, configuration, acceptance artifact, or Git state was edited. Writes are limited to this review, its checker outputs, the assigned external R04 reports, and owned `target/r04` evidence. No capability, firewall, or loopback exemption changed; no personal profile or external network was used.

## Findings

### R04-F1 — P1 / CRITICAL / required: exact loopback denial and final smoke are unmet

`crates/app/src/worker.rs:569-576` correctly requires a failing child whose last marker is exactly `SANDBOX_PROBE app=1 read=5 write=5 network=10013`. The actual integrated run instead records:

```text
SANDBOX_PROBE_FILES app=1 read=5 write=5
SANDBOX_PROBE_ERROR operation=Winsock WSAPoll loopback connect timed out with SO_ERROR=0
OpenSequoia: AppContainer exact denial observation failed: exit=Some(1) ...
```

The oracle therefore has **24 passing assertions and four unmet assertions**: `sandbox smoke exit`, `actual helper loopback WSAEACCES 10013`, `broker exact denial/token observation`, and `complete sandbox smoke`. There is no actual `SANDBOX_SMOKE_OK` or `S03C_PROBE_OK`. `accepted=0`, an unchanged fixture, and a timeout with `SO_ERROR=0` do not satisfy the required `10013` observation.

**Source diagnosis:** R04 found no concrete implementation defect that explains this result. The same staged helper and same listener demonstrate ordinary IPv4 loopback success. The AppContainer path reaches the Winsock poll after file checks. `WSADATA`/`SOCKADDR_IN`/`WSAPOLLFD` layouts, constants, port/address byte order, nonblocking setup, two-second bound, `SO_ERROR` query, socket close, and `WSACleanup` all match the installed SDK/windows-sys declarations. An immediate or asynchronous `10013` is returned as such; timeout is explicitly an error. The present evidence supports unresolved host-specific AppContainer/network filtering behavior, not a proven source fix and not a denial claim.

**Required action:** keep S03c blocked and remote browsing disabled. Diagnose the Windows filtering behavior with bounded same-source evidence on supported hosts. Do not relabel timeout, add capabilities, alter firewall/exemptions, or weaken the oracle.

### R04-F2 — P3 / INFORMATIONAL: referenced S03b design document is missing

`crates/app/src/windows_sandbox.rs:3` cites `docs/discovery/appcontainer.md`, but the file is not in the frozen tree. This prevents reviewers and maintainers from inspecting the intended integrated S03b API/design contract. Integrate the intended document with provenance or correct the reference in a later reviewed change. This documentation gap did not prevent direct review of the implementation and installed ABI declarations, and it is not counted as a new implementation defect.

No additional concrete unsafe, ACL/token, fail-open, handle-lifetime, or remote-exposure defect was identified in the scoped implementation.

## Snapshot and evidence binding

`git rev-parse HEAD` returned the full snapshot above; the worktree was clean before owned ignored outputs. `git merge-base HEAD 2027b227...` returned the stated base. The complete scoped diff was captured as `target/r04/scoped.diff` (SHA-256 `3a51acec85a882d9431d6657822458442d55c5358275636c05cef0b9229be017`).

The following independently computed hashes match `sandbox4-acceptance.json` exactly for every input that file records:

| Input | SHA-256 |
|---|---|
| `crates/app/src/windows_sandbox.rs` | `1ae44f9d316e4796cd88faceba48b412fb86e9d45f86164c7b6955f0195efc97` |
| `crates/app/src/windows_process.rs` | `65f42fea9b302858076d5046767df11b708b0a9c97e6689fd5ac05bac5cd246d` |
| `crates/app/src/worker.rs` | `802f6c5274cff59711da028d0e2ea4e38306638b6a823791fa1431d88fb09b1d` |
| `crates/app/src/main.rs` | `2c06b3551678656d80e4fac74989cb003c33a73686c91babaf3569d2cf700d52` |
| `crates/app/src/input.rs` | `25106af641986a62785d3b7f6bc2ce540e2031f6a921cf7dacadb3bb2be15a60` |
| `crates/app/Cargo.toml` | `e8d6ef76b14e277613b4acc1d02bd3437b7b1046da0f757b947b596c338c49cf` |
| `Cargo.toml` | `1caebaa3f3b1ce49737f390baad18c1daa33a2716cbdf1c0963d357ad12548dd` |
| `Cargo.lock` | `e801e3ec0b12e1d134834a119102c4ac270d328818683c5c7cdb4d11b5852707` |
| `evidence/verification/sandbox4-acceptance.py` | `f71a782718153f585aef611a3fb91fbf79bb7f32149537b9a112b40f9b1c82b3` |
| `evidence/verification/sandbox4-process-helper.rs` | `6f972a377381fd158a2004dccd67f240bb425e61bf19fb32bb2e474690ea7c72` |

Reviewed evidence fingerprints are `dd9ee9c9415ab8bd1bfc19369a12f9a85fdba6483279785209d24dcfdee9754c` for `sandbox4-acceptance.json` and `15aa1b4ecf0d3233ae0f80ca731dfe369617c8cebe208601a8526b34c169be92` for `sandbox4-acceptance.log`.

The acceptance record's historical execution revision is `22abfd5604c13e97a89ea416efd2154e75c50257`, not the review HEAD. Reuse is justified only because all recorded input hashes match and `22abfd5..b3e3ccc` changes no production engine/IPC/fixture/`.cargo` input; its remaining changes are CLI formatting and engine tests. The checker enforces these conditions. R04 did not treat the historical revision string alone as proof.

## Unsafe and security invariant review

| Boundary | Source-derived conclusion and evidence limit |
|---|---|
| SID storage and token buffers | `Sid` uses initialized `u32` backing with a 68-byte cap. `TokenBuffer` uses aligned `usize` backing with a 64 KiB cap. Length, subauthority count, pointer-within-buffer, and integrity authority/RID checks precede SID use or unaligned C-structure reads. OS SID/ACL outputs have single `FreeSid`/`LocalFree` owners. No out-of-bounds or alignment defect found. |
| Fresh profile and staging ownership | A bounded generated ASCII name is accepted only after exact successful creation; collisions fail closed. Only the fresh profile confers deletion ownership. Run roots use `create_dir`, never reuse an existing directory. Owner receipts and explicit cleanup bind the generated name/root. Drop-time cleanup errors are intentionally not success evidence. Same-user replacement/TOCTOU and arbitrary-crash discovery remain production gaps. |
| DACL contents | `SetEntriesInAclW` starts from a null old ACL and `SetSecurityInfo` uses a protected DACL. Entries are exactly current-user `FILE_ALL_ACCESS`, LocalSystem `FILE_ALL_ACCESS`, and optionally the generated AppContainer SID with `FILE_GENERIC_READ | FILE_GENERIC_EXECUTE`; inheritance flags are zero. Run root, stage, and staged executable receive SID RX. Controls, deny fixture, and owner receipt have no AppContainer ACE. No Everyone/ALL APPLICATION PACKAGES grant exists in this code. The oracle checks behavior, not DACL contents through a separate security-descriptor query. |
| Path and denial-target confinement | Trusted source and staging are canonicalized/bounded, final objects are opened with reparse-point flags, and generated parent/identity checks constrain cleanup. The denial helper requires an absolute generated `target/sandbox4/runs/<name>/controls/deny.dat` shape and a nonzero port; Winsock always uses `127.0.0.1`. Checks reduce but do not eliminate same-user ancestor/rename races; this is not a secure installer or arbitrary hostile-local-user boundary. |
| Attribute storage and lifetimes | Attribute storage is initialized/capped and aligned with `u128`. HANDLE/JOB arrays, their handle owners, `SECURITY_CAPABILITIES`, and the SID allocation remain live through `CreateProcessW` and attribute-list deletion. The generated SID has null capabilities, zero count, and zero reserved field. No premature free or unstable backing found. |
| Handles and stdio | The production sandbox launch has exactly three HANDLE_LIST stdio entries. Parent pipe ends and Job are noninheritable; child ends/NUL are inheritable. Only the explicit development control adds a sentinel or a two-process Job. No parent token/process/thread handle is intentionally inherited. |
| Suspended pre-resume policy | Sandbox creation includes `CREATE_SUSPENDED`, creation-time Job assignment, and the security attribute. Before the sole owned initial thread resumes, code requires AppContainer=1, exact generated SID equality, exact Low integrity RID 4096 with the mandatory-label authority, zero capabilities, matching process ID, and owned-Job membership. |
| Fail-closed termination | Query, identity, path, attribute, or resume failure terminates the owned Job and performs a bounded process wait. Cleanup uncertainty is included in the error; there is no ordinary fallback after sandbox failure. Ordinary launches are separate trusted development controls and the current normal application path, not a sandbox fallback. |
| Process/Job/profile/stage lifetime | Job kill-on-close, active-process and 256 MiB development limits, bounded supervisor kill/reap, pipe cancellation, and explicit profile/stage cleanup are preserved. Normal probe success requires the last Arc to be released and cleanup to succeed. Parent-death evidence opens the child observation handle before killing the parent and recovers only after observed termination. Arbitrary crash timing/orphan discovery is not covered. |
| Winsock | Successful `WSAStartup` and valid socket each have one guard; declaration order closes the socket before `WSACleanup`. Calls use initialized exact-size buffers and immediate thread-local Winsock errors. A two-second poll is bounded. Timeout/`SO_ERROR=0` is not converted to `10013`. |
| CLI and remote exposure | Sandbox smoke is explicit opt-in. The internal denial worker accepts exactly a bounded absolute local path and decimal nonzero loopback port, not a host/URL. Existing URI/share/device rejection remains. No default remote input, network browser, ordinary fallback, or remote enablement was introduced. |

## Positive controls and strict-handle assessment

The positive controls are meaningful within their stated boundaries:

- Ordinary staged rendering and AppContainer rendering return the same real display. Suspended token evidence is `app=1 sid_equal=1 integrity_rid=4096 capabilities=0`.
- The ordinary denial helper reads and rewrites the exact synthetic fixture and establishes real loopback with one synthetic byte. AppContainer read and write both return OS error 5; the broker rechecks unchanged bytes and observes no accepted connection.
- A two-process descendant control starts and is reaped; the one-process Job returns quota error 1816. This distinguishes quota enforcement from a generic spawn failure.
- The included sentinel reads the exact 32-byte marker. The otherwise identical excluded sentinel exits with `0xc0000008` (`STATUS_INVALID_HANDLE`) and no graceful marker.

For the strict-handle case, the included control proves that the same staged helper can read the marker when HANDLE_LIST contains the sentinel. The excluded run's strict invalid-handle termination is consistent with Windows AppContainer strict handle checking when the numeric sentinel was not inherited. It is useful evidence of the intended exclusion on this host, not a graceful `NOT_INHERITED` observation and not a direct enumeration of the child handle table. It does not prove exclusion of every possible handle on every Windows version. The source's exact three-entry list supplies the remaining scope argument.

The no-accepted-socket observation is weaker than exact `10013`: it shows no completed accepted connection in this run, not the required OS denial semantics. R04-F1 remains open.

## R04-owned focused probe

One focused new case justified execution: the integrated helper retains an external Arc while testing Job close, so it does not exercise `ContainedChild` as the final sandbox owner. R04 wrote `target/r04/direct-drop-probe.rs` (SHA-256 `907e37a6b368c5b0350b45895b86bc6caefb11da70d849b6ae02cef785b84891`) including the product modules read-only. It opened an observation handle, dropped the external Arc, directly dropped the child, checked immediate staging state, waited at most two seconds for child death, and attempted owned recovery only if needed.

Commands actually run:

```powershell
cargo build --target-dir target/r04/cargo -p opensequoia --locked --offline
rustc --edition=2024 target/r04/direct-drop-probe.rs --extern windows_sys=C:/Users/deepa/OneDrive/Dokumen/ChatGPT/opensequoia-crew/review-s03c/target/r04/cargo/debug/deps/libwindows_sys-e0551551b6ff6ca1.rlib -L dependency=C:/Users/deepa/OneDrive/Dokumen/ChatGPT/opensequoia-crew/review-s03c/target/r04/cargo/debug/deps -o C:/Users/deepa/OneDrive/Dokumen/ChatGPT/opensequoia-crew/review-s03c/target/r04/direct-drop/target/sandbox4/direct-drop-probe.exe
C:/Users/deepa/OneDrive/Dokumen/ChatGPT/opensequoia-crew/review-s03c/target/r04/direct-drop/target/sandbox4/direct-drop-probe.exe
```

Build and probe exited 0. Actual output:

```text
DIRECT_DROP_BEFORE root_exists=true
DIRECT_DROP_OBSERVATION immediate_root=0 immediate_binary=0 child_reaped=1 recovery=not_needed final_root=0
```

The run directory had zero remaining entries. This single Windows 11 x64 run did not reproduce a final-owner cleanup race and does not prove that wait-free cleanup is universal. Full command/result text is retained in `target/r04/direct-drop.log`. No loopback rerun or broad suite was performed.

## Reused runtime evidence and test limits

R04 read and hash-bound the actual integrated acceptance log rather than rerunning it. It records successful application/helper builds, ordinary process smoke, real renderer/token checks, exact file error 5, positive controls, Job/quota/handle/cancellation checks, cleanup and parent-death recovery, plus the four failures above. The historical `root3-sandbox-cli-red.log` is earlier red evidence, not the current result.

R04 did not independently rerun workspace tests, Clippy, formatting, the full sandbox oracle, external networking, or other Windows architectures. Author-reported generated-overlay test/Clippy logs are not independently executed R04 evidence. The focused Cargo build is the only build run by this reviewer.

Material limits:

1. No direct security-descriptor oracle independently enumerates the resulting DACLs; exact ACE contents are source-derived, while behavior is runtime-checked.
2. Only one synthetic file and IPv4 loopback listener are covered. There is no all-filesystem, all-network, device, arbitrary pipe, registry, or exploit-resistance claim.
3. No live process mitigation-policy query, startup mitigation matrix, site/origin isolation, authenticated capability IPC, TLS/origin/storage policy, or production renderer integration is established. The ordinary application renderer still has ambient user rights; remote content remains disabled.
4. Parent-death recovery is supervised with a known owner receipt after metadata handoff. A crash before receipt disclosure or a later unattended orphan sweep is not tested. Drop-time failures and same-user TOCTOU races remain production work.
5. Only the current Windows x64 host ran. Strict-handle and network behavior can differ by Windows build/policy. Finite positive controls do not prove universal handle tables or host-independent denial.
6. Production installation/staging identity, secure updates, downloads/extensions, full dependency unsafe review, and threat-model acceptance remain open.

## Checker and handoff

`evidence/verification/sandbox-review-check.py` binds the snapshot, reviewed source/evidence hashes, acceptance assertions and command results, findings JSONL, this report's machine fields, external summary, and focused probe log. It prints `SANDBOX_REVIEW_OK` only when there are zero unresolved critical findings and zero unmet required assertions and both reports agree. R04 ran `python evidence/verification/sandbox-review-check.py`: every consistency check passed, then it exited **1** with `evidence_consistent=True critical=1 required_unmet=4` and `SANDBOX_REVIEW_BLOCKED unresolved_critical=1 required_unmet=4`; it did not print the success token. Generated results are in `sandbox-review.json` and `sandbox-review.log`.

Assigned outputs:

- `docs/reviews/appcontainer-probe.md` (this report)
- `evidence/verification/sandbox-review-check.py`
- `evidence/verification/sandbox-review.json`
- `evidence/verification/sandbox-review.log`
- `../review/r04-findings.jsonl` relative to the crew root
- `../review/r04-summary.md` relative to the crew root
- owned supporting evidence under `target/r04/`

Next executable action: diagnose and satisfy the exact loopback gate without weakening policy, restore the missing S03b design record, then perform the required root/independent revalidation on a new frozen source hash. Preserve every production gap above.

## Post-review integration addendum

Added by root on 2026-11-24. This addendum does not replace or upgrade the R04 verdict above; it records the focused changes made after the reviewed snapshot and the current evidence state.

- Integrated reviewed fix `0e2d083` as `98d760c`: the parent now cfg-gates the Windows-only sandbox smoke arm, and the oracle requires parent-owned verified markers that are printed only after the child token policy and exact file/network markers validate. The denial observation is no longer printed before it is earned.
- Added `54289fd`: the fixture/file-denial observations are emitted after the child file marker and token policy validate but before the exact loopback marker is required. This keeps the file evidence visible when the host fails only the network gate; it does not relax the network or smoke assertions.
- Focused validation on `54289fd`: `cargo fmt --all -- --check` exit 0; `cargo clippy --workspace --all-targets --locked -- -D warnings` exit 0; `cargo test --workspace --locked` 72 passed, 0 failed. Log: `evidence/verification/root4-gates.log`.
- Fresh `python -B evidence/verification/sandbox4-acceptance.py`: 24 passed and 4 unmet. The child still reports `SANDBOX_PROBE_ERROR operation=Winsock WSAPoll loopback connect timed out with SO_ERROR=0`; there is no `SANDBOX_SMOKE_OK` and no `10013` claim. File denials, unchanged fixture, zero accepted connections, token checks, lifetime and cleanup assertions pass. Log: `evidence/verification/sandbox4-acceptance.log`.
- Current state remains **BLOCKED** on the exact loopback `WSAEACCES 10013` observation and final smoke. No acceptance criterion, capability, firewall, or loopback exemption was changed.
- The checker files (`sandbox-review-check.py/.json/.log`) remain pinned to `b3e3ccc` and are historical review artifacts. They are not a current pass and were not rewritten to accept the post-review tree.
