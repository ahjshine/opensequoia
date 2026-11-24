# I02r independent GUI packaging source review

Date: 2026-11-23 (Asia/Calcutta). Reviewer: gui_review, separate from root and the application/engine authors. Scope: release subsystem attribute, `main.rs` mode dispatch and worker stdio consumer, `scripts/verify.ps1`, `scripts/package_dev.py`, `.github/workflows/windows.yml` and README packaging claims. The candidate is the selected uncommitted wave 2 source on `codex/foundation`; no Git remote is configured.

**Verdict: approve the source change for the local development GUI milestone, with the limits below.** No critical or required source defect was found in this scope. Final artifact identity, PE fields, archive membership and package reproducibility are I02a/Craft gates, not established by this source review. No production browser, release installer, full accessibility, sandbox or performance verdict is implied.

## Method and evidence boundaries

Applied the installed Agent Skills code-review workflow and imported gstack `review` 1.0.0/checklist/plan-completion guidance. The canonical `C:/Users/deepa/.claude/skills/gstack/bin/gstack-skill-start` is absent, confirmed with `Test-Path`; documented degraded mode was used. No gstack helper receipt, telemetry, automatic gate scan, outside-model review or remote CI execution is claimed. Root's explicit ownership/frozen-source brief governs edits; this reviewer changed only this report and `gui-review-*` evidence.

Read the complete selected source diff and consumers, then checked critical gate/injection/enum/concurrency concerns before packaging/version/CI/ergonomics concerns. Used TASKS.md I02r acceptance as the task plan. Artifact bytes were not parsed, packaged or executed by this reviewer. The existing successful `root2-gui-*` runtime logs are reused evidence, clearly distinguished from checks independently executed here.

`evidence/verification/gui-review-source-hashes.json` records an independent comparison of all **27** input hashes from `root2-gui-verification.json`: **zero mismatches**. It separately fingerprints main, verification/package scripts, CI and README. This makes the reused GUI verification applicable to the current consumed source; any later change requires checking the relevant fingerprint again.

## Findings and source conclusions

1. **Observed source:** `crates/app/src/main.rs:2` applies `windows_subsystem="windows"` only for Windows builds without debug assertions. The checked default release profile has debug assertions disabled; debug builds retain console behavior. No engine, trust boundary, input acceptance, IPC format or browser functionality is replaced by the attribute.
2. **Derived from source, corroborated by reused runtime logs:** `windows_process.rs:411` passes `STARTF_USESTDHANDLES` with the three explicitly created child stdio handles; `worker.rs` consumes `io::stdin()/stdout()` for the bounded protocol. The GUI attribute does not remove those explicitly provided pipes. `root2-gui-process-smoke.log` records valid/malformed/restart exchanges with three children reaped. Headless inspection and native child-rendered paint/application logs also contain their expected success markers.
3. **Observed source:** `verify.ps1:17` retains format, all-target locked Clippy with warnings denied, workspace tests, locked release build and actual inspect/process checks. Native paint and application checks run unless the explicit `-SkipWindow` development option is supplied; their execution state is recorded. The alternate output directory avoids overwriting an in-use executable. Prefix and target traversal validation occurs before any Cargo or build invocation. No acceptance criterion or failing gate was removed by the reviewed script changes.
4. **Observed source:** `package_dev.py:14` accepts a selected executable as a structured argument, resolves its path, derives the numeric three-component package version from locked Cargo metadata and explicitly includes only executable/LICENSE/README/notices. It uses an argument array for Cargo without a shell. Fixed file order, ZIP timestamp, platform and permission bits make output independent of filesystem timestamps for identical bytes and the same Python/zlib runtime. Existing extraneous files in the output directory are outside the explicit ZIP member list.
5. **Observed source and limitation:** package `version`/`target` labels come from current source metadata and a fixed x64 target string; the script does not prove that an arbitrary `--executable` matches either label. Final packaging must pair the selected executable with matching verification inputs and Craft/Windows identity evidence. This is a documented development provenance limitation, not an acceptance of mislabeled artifacts. I02a owns that check for this concrete GUI build.
6. **Observed source:** CI retains pinned action/toolchain/auditor versions and locked quality/advisory checks. The wildcard upload path matches the version-derived development ZIP. A clean hosted checkout creates the current package; hosted CI does not execute interactive native/accessibility checks, and the workflow says so. This reviewer did not run GitHub Actions.

Nonblocking observations for later development work:

- **P3 GUI diagnostics (`main.rs:22`):** startup errors still use only `eprintln!`. A release GUI launched without inherited stderr can exit without a visible explanation, for example after invalid startup options or native-window initialization failure. Add an appropriate native startup-error surface while keeping worker/headless failures on their protocol/exit path. This is a source-derived ergonomics gap; this reviewer did not inject a native startup failure or claim a runtime reproduction.
- **P3 metadata wording (`package_dev.py:55,65`):** the ZIP sets Unix permission bits `0644`, but does not explicitly set the Unix regular-file type bit. All supplied entries are ordinary files without trailing directory separators, so this does not defeat Windows extraction or deterministic bytes. Describe these as permission bits, or include `stat.S_IFREG`, rather than calling the encoded value a full Unix regular-file mode.

Neither observation changes the retained browser scope or permits unsafe remote browsing. Source-only absence searches cannot establish that forbidden bundled code is absent from a binary.

## Checks independently executed

Environment: this Windows host, PowerShell, installed Python. No build or native application was launched concurrently with artifact investigation.

| Check | Result | Evidence |
|---|---|---|
| Compare `root2-gui-verification.json` hashes with `Get-FileHash` of each consumed source input | 27 checked; zero mismatches | `gui-review-source-hashes.json` |
| Parse `scripts/verify.ps1` with `System.Management.Automation.Language.Parser::ParseFile` | no parser errors | `gui-review-script-checks.log` |
| Invoke verify with `-EvidencePrefix ../../escape -BuildTargetDir target` | rejected `invalid evidence prefix` before Cargo/build | same log |
| Invoke verify with `-EvidencePrefix gui-review-test -BuildTargetDir ../outside` | rejected `build target must remain` before Cargo/build | same log |
| `python -c 'import ast,pathlib; ast.parse(pathlib.Path("scripts/package_dev.py").read_text(encoding="utf-8")); print("PYTHON_AST_OK")'` | exit 0; AST parsed without executing package main | same log |
| Direct source/diff review and `rg` consumer tracing | no required defect identified in scoped change | source paths above and this report |

The negative script probes were caught only when their messages matched the expected rejection; unexpected acceptance/errors would fail the reviewer command. The source-hash command fails if any mismatch exists.

## Reused checks and remaining limits

Read, but did not rerun, the applicable `root2-gui-*` logs: Clippy succeeded; **35 app + 9 engine + 10 IPC = 54 tests passed**, zero failed/ignored; locked release build succeeded; inspect reported 49 nodes/169 text runs with one explicit unsupported-feature diagnostic; process smoke passed; native paint recorded child rendering; app smoke recorded local navigation/history/tabs/native controls and 11 paints. R03 already independently reviewed and executed the broader local application milestone. Frozen-source duplicate runs were avoided.

No WPT/Test262/Wasm suite, hostile remote browsing, AppContainer/token/file/network denial, TLS/origin authentication, signing/install/update, clean-machine CRT, assistive technology, all physical shortcuts, comparative memory/startup/frame measurement or arbitrary dependency/native-code completeness check was run by this reviewer. Full required gates remain pending. EXE/ZIP hashes and PE/resource/signature facts must come from I02a's recorded artifact investigation.

## Handoff

I02r deliverables: this report, `evidence/verification/gui-review-source-hashes.json`, `evidence/verification/gui-review-script-checks.log`. No implementation/config/package edits or commits. Root may integrate this scoped source approval once I02a reconciles the final GUI EXE/ZIP and Craft ledger. Preserve the unsigned development label and all unresolved security/parity/accessibility/performance gates. Next executable tasks: final artifact acceptance; AppContainer denial probes with positive controls; origin/TLS/standards work. Durable learning: a GUI subsystem binary can retain child protocol pipes through explicit Win32 stdio handles, but source and actual GUI worker verification must both be checked; console draft evidence must not be reused as final GUI identity.
