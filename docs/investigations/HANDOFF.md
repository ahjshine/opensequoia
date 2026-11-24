# D06 static artifact milestone and handoff

Owner: `/root/rust_feasibility`. Date: 2026-11-22. The static evidence batch is complete; whole-browser acceptance and embedded installer payload correspondence remain incomplete. No target code was executed.

Read [craft-report.md](craft-report.md) for the generated findings, confidence labels and full evidence IDs. Durable project: `evidence/artifacts/brave-opensequoia.cany`. Export: `evidence/artifacts/craft-evidence.json` (19 records, 3 open unknowns). Thirteen settled findings all reference IDs present in the export; `craft-record-check.json` records that cross-check. This validates references, not correctness/security/performance of a browser.

## Observed artifact identities

| Artifact | File bytes | SHA-256 / observed identity |
|---|---:|---|
| Before-CFG OpenSequoia snapshot, `craft-opensequoia-s02.exe` | 904704 | `6d7bde699288ce03570ac30e0e35e42a585681393fadf1449fba3ec2ebcfcccd` |
| Final OpenSequoia snapshot, `craft-opensequoia-final.exe` | 908800 | `930a4415a37137ad33ce77e4c82c134aa1ae0be5dcae14bc8449104ffb7042b8` |
| Final development ZIP, `raw/opensequoia-0.1.0-win-x64.zip` | 359254 | `e1f6b22bb9f84113fc3f873e57a682d7b83cceb09936d083c172fc10b68183bf` |
| Installed Brave `brave.exe` | 4624976 | `eb7bdd6794feef543c70ed73db5a408636b9f0b86f6977609e8034b1f6da932f`; official version158.1.100.58 |
| Installed Brave `158.1.100.58/chrome.dll` | 338606160 | `be842bf3a479d88dfc1fdc4ef4cbb52589ff1de0b34259c5921cbd6489ddcf1f`; official version158.1.100.58 |
| Selected standalone setup | 173087584 | `2802553c89831effe624bd1b0dd8e39c463e03a1224d557441350a5586e559ce`; updater-wrapper PE version1.3.361.151 |

The setup file's browser-release association comes from the root's pinned release URL/digest, not its updater PE version. Installed browser/core resources have LastChange `bd6d26d1bced03c17a98baa86d2cd3425d543d1b`; source/build correspondence beyond that requires the source baseline records and payload validation. The wrapper is x86; the installed browser and OpenSequoia are x86_64. Names alone were not used as architecture proof.

## Findings that affect integration

- Final PE CFG is true, whereas the earlier snapshot was false. NX/PIE/high-entropy VA flags are true; branding remains OpenSequoia0.1.0.0 with `asInvoker` and PerMonitorV2 manifest. Root used `.cargo/config.toml` with `-C control-flow-guard=yes`; final observed hash matches the frozen root build. The 4096-byte difference is not a controlled attribution or performance result. Final layout evidence: `ev_5c812b8afc08c37a8703187d37d55c51dc6ec0d6532cc81a074ac6d89b5ff51a`.
- The complete final import table has146 rows/12 recorded library names (KERNEL32/kernel32 case variants). It includes Windows API services and VCRUNTIME140/CRT dependencies. Imports and missing symbols cannot prove absence of statically linked native implementation or runtime loads. Final import evidence: `ev_7bde15503add490eaab3b678e0ba974cb9c876725ef2b930f407dcf6733d9b5c`.
- Final ZIP has exactly four members: EXE908800, LICENSE17099, README2585 and THIRD_PARTY_NOTICES322482 bytes, total1250966 unpacked file bytes. Member EXE hash equals the analysed final snapshot; no ZIP contradictions were reported. This is a development package, not an installation/update/recovery or clean-machine prerequisite pass. Evidence: `ev_ec8eed8efe42aa4f623e196215e20703eff68bf5c2ac5f9693bf7c9285e3f8b0`.
- Brave's selected version-folder inventory contains377 files/455 entries/1146695025 hashed file bytes. `Installer/chrome.7z`569442764 bytes is larger than chrome.dll338606160 bytes, correcting the initial largest-file hypothesis. Four VPN PE files total24467776 bytes. These support bloat/infrastructure classification, not a runtime activation or RAM/startup claim. Evidence: `ev_99e11378acc62a330777ece005827fef264a7769cc3896b420f05870ceebc8f5`.
- Standalone resource B/102 has172883883 bytes. Its first32 bytes are consistent with an LZMA-Alone header (8MiB dictionary, declared176842616 unpacked bytes), but decompression and member identities were not validated. `core_archive_show` treats the wrapper as one PE file. Relevant IDs: `ev_026c2c08846b9676cac0840a005208228d1efe16730cb3d17a4d5c6393cea00e`, `ev_f6089e1ab3d105eb0acc8bf494da10921f662fc0568f28ebdc129064dcb78696`.
- **Signature limitation:** `native-layout` reports `code_signature:null` even for the Brave executable independently validated by the root's Windows Authenticode checks. Null is not an unsigned verdict. Root must use actual Windows signature validation for publishing/security records. This finding remains distinct from OpenSequoia's signing status.

Root should link these to BASELINE/ARCHITECTURE, FEATURE_PARITY, BLOAT_DECISIONS, SECURITY, BENCHMARKS and TASKS. No runtime size/memory/startup benefit against equivalent Brave functionality is established by this batch. Installer repair caches may be supporting infrastructure; do not delete them merely from the largest-file observation.

## Commands and tool environment

Installed Craft skill metadata version2; CLI/server0.6.1, Windows x86_64. Executable:
`C:/Users/deepa/AppData/Local/Programs/CraftAnything/v0.6.1/craft-anything-cli.exe`.

Every call used the verified `--project <absolute brave-opensequoia.cany path> --deep` prefix. User-approved Deep mode was confirmed with session-show. `--help`, `commands`, `capabilities` and each relevant subcommand's help supplied the actual schemas. No setup/reinstall, `--allow-run`, Ghidra-heavy analysis, external PE parser or MCP-session overwrite was used. Core/native built-in tools handled this batch.

Actual calls included investigation/session show; brief/plan; hypothesis record/update; target open; overview summary; native library calls; PE resources; native layout; bounded32-byte read; archive inventory; evidence browse/show; findings/unknowns/handoff; evidence export and generated report. Individual JSON results are under `evidence/artifacts/craft-*`. The full-report and export save operations succeeded (16819 and433503 bytes respectively).

CLI invocations have ephemeral target state: pass the absolute target path for each analysis tool. Project memory, journal and evidence persist. The inventory's initial one-row evidence only contained that page; evidence-browse demonstrated it, so a new bounded500-row inventory was requested to inspect all455 entries. Avoid assuming a page silently contains all records.

Tool errors are preserved: `severity=important` was a rejected input, corrected to the returned schema enum `high`. Installer native-layout returned `ev_095d...`, but finding/evidence-show could not load that ID, even after summary mode. The associated hypothesis was explicitly abandoned; settled payload findings cite intact resource/byte records. Do not fabricate/import a replacement ID or treat a returned-but-unloadable ID as evidence. No consent/policy rejection was bypassed.

## Precise next actions

1. Independently review the13 findings and19 exported evidence records against the final hash. Keep runtime security/compatibility milestones separate.
2. Establish a bounded static LZMA extraction/validation procedure for resource B/102, pin payload member hashes/versions and compare to installed/source baseline. Do not execute the installer or weaken consent. This is still unresolved work, not proof of artifact mismatch.
3. Tie final locked dependency/build/native-object/SBOM provenance to the EXE hash; verify dynamic runtime modules and clean Windows CRT prerequisites through separately authorized runtime tests.
4. Verify sandbox tokens/process/site/origin isolation, IPC capability/peer checks, signed updates and whole-process performance/conformance using actual runtime procedures. Static flags cannot satisfy these gates.

No personal browsing profile, cookie, credential or secret was opened or recorded. Only installed application files, public downloaded artifacts and OpenSequoia builds were targeted. No root records, manifests, source code or commits were changed by D06.
