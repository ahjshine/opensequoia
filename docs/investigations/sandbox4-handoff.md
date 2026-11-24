# I03a exact EXE Craft investigation handoff

Date: 2026-11-24 (Asia/Calcutta). Task: I03a. Owner: artifact_release / root integration.

## Artifact identity

- Executable: `target/i03a/release/opensequoia.exe` in the `controller` worktree.
- SHA256: `b8ae67c3b6839c1be1751bb3eb49088bf9999491b38284b58eb309e74cbc7499`.
- Size: 1,043,456 bytes.
- Source commit: `54289fd358547870b6fce3b9040a959dfad4efc5`. The executable was produced while `crates/app/src/worker.rs` was still uncommitted; the committed file is byte-identical to the build tree, and no source changed between the build and the commit. `evidence/verification/root4-build.json` records that distinction.
- Build: `cargo build --release --locked --target-dir target/i03a -p opensequoia`, rustc 1.98.1, release profile `opt-level=s`, thin LTO, one codegen unit, stripped debuginfo, panic abort.
- Cargo.lock SHA256 `e801e3ec...` and app manifest SHA256 `e8d6ef76...` are recorded in `evidence/verification/root4-build.json`.

## Method

The installed Craft Anything 0.6.1 CLI was used in Deep static mode with project `evidence/artifacts/sandbox4-opensequoia.cany`. The CLI keeps target state only for one call, so every analysis call passed the absolute path. Recorded calls and outputs are under `evidence/artifacts/sandbox4-craft-*.json`; the driver is `evidence/artifacts/sandbox4-craft.py`.

Observed Craft evidence:

- Overview: `ev_d1415ce57072b4b4827b09c86e89b3015be36e6e802685febd7630c0bc468b49`.
- Layout/hardening: `ev_9a529d3bb5ea667ee02508b0100d5011ec72493a960ac1bd33edad2480bfb32d`.
- Sections: `ev_66f94bf6e58ad91467cbb85bc0da846ccd95e1bc4a2b3adfd28838c5b73079b1`.
- Resources: `ev_273186e83b0e8fea1dd8a9b7a4964dc0e6b012e45d4ccee5ee3ce2cb0902a0b9`.
- Imports (new): `ev_5e4aee0fe23a252922a7a0b06a1be7b3f07573ce857812280968cdc459b22c60`.
- Imports (prior `d80778d7` snapshot): `ev_2e484e3ca3417b503945f2b7b5bbe3b34e8b6551372345d8280b7c7f2cedfa26`.

## Settled findings

1. The exact bytes are a Windows PE named `opensequoia.exe` with the hash above; both the overview and layout records bind it.
2. It is an x86_64 PE32+ `windows-gui` executable with six sections, entry `0x140080850`, and NX, PIE, high-entropy VA and control-flow guard true. `code_signature` is null; that is not an unsigned verdict.
3. It imports 199 functions from 16 libraries versus 162 from 12 in the prior snapshot. New libraries are `WS2_32.dll`, `advapi32.dll`, `bcryptprimitives.dll` and `userenv.dll`; no script-engine or browser-engine library appears. Static imports do not establish runtime network use or remote browsing.
4. It has two resources: RT_VERSION reports OpenSequoia 0.2.0.0, and RT_MANIFEST requests asInvoker, PerMonitorV2 DPI awareness and longPathAware support.
5. Relative to the prior `d80778d7...` EXE, the new binary is 97,280 bytes larger, adds 166 discovered functions and 37 import rows, and keeps the same subsystem, hardening flags and product/version metadata.

## Checks

- PowerShell SHA256 and size repeated after the build; the digest matches the Craft target record.
- Workspace `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings` and `cargo test --workspace --locked` passed; 72 tests, 0 failed. Log: `evidence/verification/root4-gates.log`.
- The target was never executed for this investigation and `--allow-run` was not used.
- No package ZIP was produced or analysed for I03a; the prior GUI ZIP remains the packaging record for the older snapshot.

## Limits and unknowns

- Static analysis only. No runtime security, sandbox, compatibility or performance claim follows.
- 58 indirect jumps were unresolved and local symbols are stripped; code reached only through computed pointers or at run time may be missing.
- Imports do not prove the absence of statically linked code or of later `LoadLibrary`/resolver use.
- Binary reproducibility, Authenticode signer validity, runtime behaviour and the AppContainer `10013` gate remain open. See the Craft report's open questions.

## State

I03a static investigation is complete for the exact EXE above: brief, plan, hypotheses, four findings, unknowns, export, report and this handoff exist with loadable evidence IDs. It does not verify the application at runtime and does not unblock S03c/R04.

Next executable action: root review of this handoff and report; then keep the release unsigned/runtime-unverified until a signing intake and authorized runtime host are available.