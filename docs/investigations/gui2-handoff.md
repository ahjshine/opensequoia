# I02a GUI development package handoff

Date: 2026-11-23, Asia/Calcutta. Owner: artifact_release. This is a bounded local development application milestone, not a production browser release.

## Identity and checks actually run

- All `root2-gui-verification.json` consumed-input SHA256 values matched current files before packaging and analysis. The pre-existing 54-test/runtime verification was checked by fingerprint; it was not re-run by this agent.
- Windows file metadata: OpenSequoia, FileVersion 0.2.0.0, ProductVersion 0.2.0. `Get-AuthenticodeSignature` returned `NotSigned`; Craft's signature fields are not a Windows trust verdict.
- Executable: `target/gui/release/opensequoia.exe`, 946176 bytes, SHA256 `d80778d7dbaa7f3a520b33e88ab3cb8346cce1c8c31749cba19653ede0511f3e`.
- Ran `python scripts/package_dev.py --executable target/gui/release/opensequoia.exe` twice. Identical-input package SHA256 values matched: `743c6594a6e72f0dffdcfbf07537b4900659da35d018933eef39df21d871ff05`, 379375 bytes. ZIP: `evidence/artifacts/raw/opensequoia-0.2.0-win-x64.zip`.
- Package member file bytes total 1289128. This is neither allocated installed size nor a comparable full-Brave footprint measurement.

## Craft findings and evidence

Resumed installed Craft 0.6.1 skill version 2 and existing `evidence/artifacts/brave-opensequoia.cany`. The MCP session had no project and exposed no project-open operation, so documented CLI `--project` commands preserved the durable ledger. Deep mode was already enabled. No setup, install, runtime target launch, consent bypass or MCP reconfiguration occurred. Only needed native/formats/archive capabilities were used; the CLI exposes the full catalog without a separate toolset-enable command.

| Finding | Confidence / observed scope | Citable evidence |
|---|---|---|
| fnd_18 | Observed: PE32+ AMD64, Windows GUI subsystem 2, HIGH_ENTROPY_VA/DYNAMIC_BASE/NX_COMPAT/GUARD_CF. Header adapter first 250 of 22890 lines examined; claims concern the visible header fields. | `ev_a5ef3c46551c09321e8f1df477403bdfc68782d330ee8a107721730a8f3b1e3b` |
| fnd_19 | Observed: complete two-resource inventory; OpenSequoia0.2.0.0 version and asInvoker/uiAccess false/PerMonitorV2/longPathAware manifest. No icon group in this resource tree. | `ev_7c864322896125c053adf7e03d6f2f6488bcb36bc41dc676c066a3e375509527` |
| fnd_20 | Derived: complete 162 normal imports from 12 recorded library names match console-draft semantic function/library/kind/hint inventory; import slots change. OS/CRT imports include VCRUNTIME140, Job APIs and UI/stdio services. | `ev_f97f3c572503836ee601339717f5779c4f277218b6637baef179d3d4a441212b`; prior draft `ev_751a26dab5578a14416d3b7ad83c50792deaae9b46901ef57c64836d041ee258` |
| fnd_21 | Observed: complete four-member ZIP inventory; GUI executable hash matches, plus LICENSE17099bytes, README3371bytes, notices322482bytes; no ZIP contradictions. | `ev_5da17a302b1ea9c68081ad590c783575f88bc1474663a074758898275d34f2bb` |

Each current evidence record loaded through `core_evidence_show` and was accepted by `core_finding_record`. The final evidence-integrity JSON checks all settled finding citations against the exported ledger, with zero missing IDs. Four current findings are fnd_18 through fnd_21. GUI/package hypotheses are supported. Literal full-import-row equality was refuted because slot addresses differ; the narrower semantic equality is separately documented as a derived finding.

## Limits and next executable tasks

The earlier app2 console draft is historical: its subsystem3 and old EXE/ZIP hashes are not the delivered GUI package. Older native-layout/overview returned IDs that were not loadable; their failures remain recorded, and no current finding uses those IDs. Static imports cannot exclude linked native code or future runtime-loaded dependencies. Header mitigations do not establish sandboxing, security or performance. Clean-machine VC runtime availability, supported Windows10/11 installation, signing, secure update/recovery and removal remain unverified. AppContainer/site/origin/TLS, full standards, extensions/sync/media/passwords/accessibility and comparative performance gates remain unmet.

Root next maps the committed source checkpoint to GUI verification inputs and final package hashes, reconciles root records, accepts independent review and commits. Any future change to executable or packaged README/license inputs needs a newly identified package and artifact batch. Resume Craft with investigation/session show; preserve historical exports and read the live plan/handoff first.

Artifacts: `gui2-*.json` preserve each exact CLI tool/params/stdout/stderr/result; `gui2-craft-evidence.json` is the complete immutable export for this milestone; `gui2-craft-report.md` is the generated cumulative report. Generated raw binaries/ZIP remain ignored and are not source implementation.
