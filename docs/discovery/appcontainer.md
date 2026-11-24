# S03b AppContainer launch contract

Date: 2026-11-23 (Asia/Calcutta). Owner: sandbox_design. **Discovery complete; implementation and runtime acceptance pending.** This document reduces source/API uncertainty. No AppContainer process was launched in this pass, no network socket was opened, and no filesystem ACL or OS profile was changed. Remote browsing remains disabled. The Windows OS implementations below are permitted documented services; no native browser engine is proposed.

The frozen process/worker inputs were read at source checkpoint `0d10c8547ddbc89cba68ce2b011780640d0a53e5`, with uncommitted application wave 2 changes captured by `evidence/verification/sandbox3-source-hashes.json`. That checkpoint alone does not identify the current source; the JSON is the authoritative SHA-256 record.

## Observed sources and APIs

`crates/app/Cargo.toml` locks `windows-sys =0.61.2`; Cargo.lock archive checksum is `ae137229bcbd6cdf0f7b80a31df61766145077ddf49416a728b02cb3921ff3fc`. Actual generated source under `C:/Users/deepa/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/windows-sys-0.61.2/src/Windows/Win32` was inspected:

| Source | Observed declaration / implication |
|---|---|
| `Security/Isolation/mod.rs:1-3` | `CreateAppContainerProfile`, `DeleteAppContainerProfile`, `DeriveAppContainerSidFromAppContainerName`, HRESULT, documented `userenv.dll` services |
| `Security/mod.rs:545` | C-layout `SECURITY_CAPABILITIES { AppContainerSid: PSID, Capabilities: *mut SID_AND_ATTRIBUTES, CapabilityCount: u32, Reserved: u32 }` |
| `Security/mod.rs:79,100,1147-1166` | `FreeSid`, `GetTokenInformation`, `TokenAppContainerSid`, `TokenCapabilities`, `TokenIntegrityLevel`, `TokenIsAppContainer` |
| `System/Threading/mod.rs:244,820` | `OpenProcessToken`; `PROC_THREAD_ATTRIBUTE_SECURITY_CAPABILITIES =131081` |
| `Security/Authorization/mod.rs:54,84,86` | SID conversion, `SetEntriesInAclW`, `SetNamedSecurityInfoW`; ACL error return is a DWORD, not BOOL/HRESULT |
| `windows-sys/Cargo.toml:141,152` | `Win32_Security_Authorization` and `Win32_Security_Isolation` add modules to the existing Rust binding crate; no additional runtime package |

Official Microsoft pages were fetched as source documentation using `python evidence/verification/sandbox3-fetch-docs.py` and `python evidence/verification/sandbox3-extra-docs.py`. Text snapshots and HTML/text hashes are recorded in `sandbox3-docs.json` and `sandbox3-extra-docs.json`. Retrieved pages contained their API sections despite generic sign-in navigation text. Relevant citations:

- [CreateAppContainerProfile](https://learn.microsoft.com/en-us/windows/win32/api/userenv/nf-userenv-createappcontainerprofile): profile is per-user/per-app; name at most64 characters using `[-_. A-Za-z0-9]+`; SID output must be released with `FreeSid`; ERROR_ALREADY_EXISTS is an HRESULT outcome, not a fresh profile ownership receipt.
- [DeriveAppContainerSidFromAppContainerName](https://learn.microsoft.com/en-us/windows/win32/api/userenv/nf-userenv-deriveappcontainersidfromappcontainername): returned SID also requires `FreeSid`.
- [DeleteAppContainerProfile](https://learn.microsoft.com/en-us/windows/win32/api/userenv/nf-userenv-deleteappcontainerprofile): close all profile file handles first; failure leaves status undetermined; deleting a missing profile returns success. Never delete a name merely because derive succeeded.
- [SECURITY_CAPABILITIES](https://learn.microsoft.com/en-us/windows/win32/api/winnt/ns-winnt-security_capabilities): exact typed AppContainer SID/capability list layout.
- [UpdateProcThreadAttribute](https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-updateprocthreadattribute): SECURITY_CAPABILITIES creates an AppContainer; HANDLE_LIST needs inheritable real handles and bInheritHandles TRUE; JOB_LIST applies during creation. **Attribute values must remain live until DeleteProcThreadAttributeList**, not merely until CreateProcess returns. AppContainer attributes require Windows8+/Server2012+; JOB_LIST requires Windows10+/Server2016+.
- [GetTokenInformation](https://learn.microsoft.com/en-us/windows/win32/api/securitybaseapi/nf-securitybaseapi-gettokeninformation) and [AppContainer for legacy apps](https://learn.microsoft.com/en-us/windows/win32/secauthz/appcontainer-for-legacy-applications-): token query determines actual AppContainer membership; low integrity and AppContainer are distinct properties. Unpackaged apps manage their profiles themselves.
- [AppContainer isolation](https://learn.microsoft.com/en-us/windows/win32/secauthz/appcontainer-isolation): resource access is capability/ACL-dependent, including granular network access. Read-only file access is less restricted than write access.
- [SetNamedSecurityInfoW](https://learn.microsoft.com/en-us/windows/win32/api/aclapi/nf-aclapi-setnamedsecurityinfow): caller must own the object or have WRITE_DAC for DACL changes; preserve owner/group and unrelated ACEs. Return ERROR_SUCCESS or exact nonzero error.
- [GetProcessMitigationPolicy](https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-getprocessmitigationpolicy): query live policies; build flags alone are insufficient evidence of enforced process policy.

Two attempted Winsock documentation URLs returned404 and are recorded, not counted as evidence. A dedicated authoritative network/loopback-policy source and live denial verification remain required.

## Derived smallest implementation

Root should enable the two existing windows-sys features above. Add a narrow `windows_sandbox.rs` module; leave worker protocol/engine safe. Separate preparation from spawning so repeated renders do not quietly acquire different authorities.

```rust
// Proposed API contract, not compiled implementation.
pub struct RendererSandbox { /* owns generated profile identity and SID */ }
impl RendererSandbox {
    pub fn prepare(staging_root: &Path) -> io::Result<Self>;
}
pub fn spawn_renderer(
    command: &Command,
    sandbox: &Arc<RendererSandbox>,
) -> io::Result<ContainedChild>;
```

`prepare` must accept only a dedicated generated staging location. The broker creates a bounded ASCII profile name containing OpenSequoia plus a nonreused nonce; retry collision by selecting another name, never treating ERROR_ALREADY_EXISTS as permission to delete/reuse an arbitrary existing profile. Profile/SID creation errors fail closed. For a deliberately persistent owned profile, derive/reuse needs a separate provenance/migration contract. The discovery probe uses only fresh profiles.

Initialize SECURITY_CAPABILITIES with the owned valid SID, null capability pointer, zero capability count and reserved zero. **No internetClient, internetClientServer, privateNetworkClientServer, broad filesystem or device capabilities.** Empty explicit capabilities do not eliminate access to resources already ACL-granted to ALL APPLICATION PACKAGES or that specific SID. LPAC/ALL_APPLICATION_PACKAGES opt-out is a separate unverified decision, not implied here.

The current AttributeList uses count2 for HANDLE_LIST and JOB_LIST. The smallest sandbox launch adds a typed setter for SECURITY_CAPABILITIES and count3. If mitigation is added, count4. SID owner, boxed/stable SECURITY_CAPABILITIES, HANDLE/JOB arrays and scalar policy storage must be declared/owned before the attribute list so their destruction occurs afterward. Avoid exposing a generic raw-pointer setter publicly. Keep initialized bounded aligned backing, exact `size_of` checks and current DeleteProcThreadAttributeList ownership.

Keep current Job kill-on-last-close, one-process quota, memory limit, explicit stdio HANDLE_LIST, sanitized loader environment, bounded absolute application path/UTF16 quoting and cancelable pipe cleanup. Add CREATE_SUSPENDED, query the exact owned child token/Job before ResumeThread, and abort/terminate/reap on failed verification. Verify TokenIsAppContainer==1, EqualSid with expected SID, low integrity and zero explicit capabilities; parse variable-sized token buffers with checked lengths/alignment and no unchecked pointer dereferences. Resume precisely the owned initial thread and check the return value; no unrestricted fallback. If query/creation/ACL/profile APIs fail, display a development error while leaving remote browsing disabled.

AppContainer cannot be assumed to execute the existing OneDrive workspace binary. Stage a trusted copy under a newly created broker-owned directory, preserve broker/user/SYSTEM control, and grant only the generated SID read/execute on that directory and file. Do not grant the SID access to workspace, Downloads, browser profiles, or ancestor directories. Do not grant ALL APPLICATION PACKAGES or Everyone merely to make startup pass. Reject/recheck reparse points and path escapes; use opened handles for ACL/file identity when practical. A safe staging race/installer contract remains required before production use. The renderer should read page bytes only from admitted IPC handles, never arbitrary filenames. A noninherited broker-side deny fixture is outside staging.

ContainedChild must retain a cloned Arc of the sandbox owner, with the owned Job closing before the final sandbox owner can delete its profile/staging. Hold profile/staging lifetime until all corresponding jobs/children and profile handles are closed. Deletion must apply only to the freshly created owned profile; explicit cleanup reports failure rather than silent success. Drop may supply best-effort cleanup but cannot provide the only evidence. No recursive deletion/move of computed paths without resolving and verifying they remain in the generated test/staging root. No ACL edits to an existing user folder.

## Mitigation follow-on

For the current headless worker, evaluate Win32K syscall disable and dynamic-code prohibition separately from sandbox bring-up. Installed SDK10.0.26100.0 `um/WinBase.h:3717,3739` and fetched UpdateProcThreadAttribute docs agree on `1u64 <<28` for Win32K disable and `1u64 <<36` for dynamic-code prohibition. These macros were not found as named constants in windows-sys source; any local named constant must cite the SDK/docs and be independently reviewed.

Query live ProcessSystemCallDisablePolicy, ProcessDynamicCodePolicy, ProcessControlFlowGuardPolicy, ASLR/DEP/strict-handle policies. Do not apply Win32K disable to the UI broker. A future Rust JS/Wasm JIT might conflict with dynamic-code prohibition; preserve retained standards scope and design a reviewed JIT policy rather than dropping functionality or quietly disabling mitigations. Current worker parses/layouts without GDI calls, so Win32K prohibition is a testable hypothesis, not an observed result.

## Acceptance before implementation: S03c isolated probe

The first runnable slice should launch the existing actual renderer under this new policy and render trusted IPC-delivered HTML, plus a dedicated Rust denial helper. Use loopback only for network testing; do not transmit browsing data, use personal profiles, or contact external services.

| Required gate | Positive control | Sandbox observation required |
|---|---|---|
| Process policy | ordinary contained helper has TokenIsAppContainer0 | AppContainer1, expected SID, low integrity, zero capabilities; owned Job membership before resume |
| Actual page path | ordinary worker renders known fixture with existing codec | sandbox worker returns same bounded width-bound display and exits/reaps; no fallback |
| Noninherited file read | ordinary helper reads generated marker in user-only DACL fixture | exact ERROR_ACCESS_DENIED5, not missing-file/path/launch failure; never log marker bytes |
| Noninherited file write | ordinary helper writes its generated fixture | exact5; fixture remains unchanged; distinguish DACL denial from low-integrity write protection |
| Required loader read | ordinary helper runs staged binary | sandbox loads/runs solely with SID-specific read/execute ACL and read-only IPC page source |
| Network loopback | same ordinary helper connects to freshly bound broker listener and transmits one fixed synthetic byte | exact WSAEACCES10013 with no accepted sandbox connection; unrelated timeout/refused/startup error fails test. Fresh SID, no exemption enabled. This alone does not establish all Internet/network denial |
| Handle authority | existing included-marker control succeeds | unrelated handle remains excluded with existing test, plus broker/profile handles never admitted |
| Process quota/lifetime | existing successful two-process control | exact descendant quota1816, Job close/parent death kills, cancellation995; re-run under actual AppContainer policy |
| Fail closed | valid SID/ACL/policy creates sandbox | invalid SID, inaccessible stage, attribute/token-query errors cannot execute an ordinary renderer |
| Cleanup | fresh uniquely owned profile/stage created | terminate/reap, explicit DeleteAppContainerProfile success, owned stage cleanup; keep failures/report limits |

All quality gates and independent unsafe/security review remain mandatory. Source tests or a handcrafted helper cannot establish production origin/site isolation, real-browser exploit resistance, secure IPC capabilities, TLS validation, download/extension isolation or final packaging. Full Windows10/ARM64 behavior remains untested; the available host is the recorded Windows11 x64 environment.

## Handoff

S03b acceptance met for source/API discovery and a dependency-ordered executable contract. No runtime security gate passed in this turn. Files changed: this document and `evidence/verification/sandbox3-*` documentation/downloader/hash records. No application, manifest, lockfile, ACL or OS profile was edited. Exact observed commands/results are recorded below in the companion handoff; two404 doc failures and one PowerShell quoting/glob failure are retained as retrieval limitations. Root should assign S03c with ownership for windows_sandbox.rs/windows_process.rs/denial helper, enable the existing binding features, implement one launch/token test, then add file/network denial controls and independent review before integrating into the renderer. Remote browsing must remain gated throughout.
