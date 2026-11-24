# S03b source discovery handoff

Owner: sandbox_design. User date2026-11-23 Asia/Calcutta; host UTC source-check timestamp retained separately. No application implementation or OS security setting changed.

Read AGENTS.md/SPEC.md/CONSTRAINTS.md/ARCHITECTURE.md/TASKS.md and ECC rust-patterns/rust-testing plus installed security-and-hardening/source-driven-development skill instructions. Applied source-first API verification, minimal ownership/contracts and explicit negative/positive test design. No ECC executable/hook, gstack helper, coverage tool or production sandbox test is claimed.

Deliverable: `docs/discovery/appcontainer.md` contains source-backed APIs, profile/SID/ACL lifetime and cleanup rules, bounded STARTUPINFOEX integration with existing Job/handle list, low privilege token validation before ResumeThread, no-capability policy, executable staging boundary, mitigation consequences and required denial-probe acceptance gates.

Commands actually executed:

- `Get-Content` required records, actual process/worker/manifests, exact installed skill instructions: success. Some initial combined outputs truncated; focused API/process sections were read afterward.
- `rg` exact windows-sys source for AppContainer/security/token/ACL/attribute declarations; source version `=0.61.2`, archive checksum from Cargo.lock: success. A broad `rg` across registry listed other unrelated packages; these were not adopted.
- `python evidence/verification/sandbox3-fetch-docs.py`: exit0; nine Microsoft API pages retrieved, one Winsock URL404. Results/hash receipts in sandbox3-docs.json.
- `python evidence/verification/sandbox3-extra-docs.py`: exit0; legacy AppContainer page retrieved; second Winsock URL404. Link and result/hash receipts retained.
- SDK source searches found WinBase.h10.0.19041.0 and10.0.26100.0 Win32K/dynamic-code constants; latter source fingerprint retained. windows-sys does not expose those macro names; no undocumented implementation constant added.
- `git rev-parse HEAD`: observed `0d10c8547ddbc89cba68ce2b011780640d0a53e5`. Existing dirty wave2 source requires content hashes in addition to HEAD.
- PowerShell Get-FileHash recorded nine input hashes; separate comparison returned exit0, nine inputs/zero mismatches. `sandbox3-source-check.json` stores UTC timestamp and result.

Retrieval failures: `rg` given a wildcard as a Windows literal path failed123; corrected to directory search with `-g`. One inline Python regex command failed PowerShell parsing before execution; replaced with a saved Python source file. Two404 network documentation attempts are unverified sources, not passes. No secrets/personal data or fixture bytes were retrieved from the host.

Acceptance met: exact available binding interfaces/features identified; authoritative API docs and inspected sources fingerprinted; first actual sandbox slice and tests have written acceptance criteria before implementation. Unmet: all AppContainer token/file/network/runtime/cleanup/mitigation acceptance, full Windows matrix and independent unsafe review. No Rust production code was written, compiled or run in this discovery task; tests were not reported as passing.

Next executable task S03c: root assigns windows_sandbox.rs/windows_process.rs/helper ownership and enables only the two existing binding modules. Implement fresh generated profile+SID, dedicated stage read/execute ACL, three-attribute launch and suspended live-token query; prove actual worker rendering and exact5/10013 denial controls before integration. Do not broaden ACLs on OneDrive/workspace/user profiles or enable loopback exemptions. Keep existing Job/handle/cancellation tests and remote browsing gate. Hold owned sandbox lifetime in ContainedChild; cleanup must report errors and never delete an unowned existing profile.
