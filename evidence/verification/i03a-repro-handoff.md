# I03a reproducibility handoff

Date: 2026-11-24 (Asia/Calcutta). Owner: root. State: **content-equivalent to the recorded artifact modulo PE build identity; the earlier "24 timestamp bytes / normalized match" claim is retracted after independent review**. Not signed and not a release-trust claim.

## Correction

An app-owned Claude Haiku 5.5 review (task `node:delegated-task:command%3Amcp%3Abcf0fc16-627d-4141-8990-7c171cba1e65%3Adelegate-task%3Ai03a-review-haiku-1`) returned **REJECT**. It was right: schema 1 zeroed every differing byte separately for each pair, so `normalized_equal` and `brepro_normalized_equal_recorded` were true by construction. The 16-byte CodeView PDB GUID was also mislabelled as a timestamp copy. Schema 2 parses the PE, masks only the COFF timestamp, debug-directory timestamps and CodeView GUID, and reports `/Brepro` differences instead of hiding them.

## Artifacts

| Item | Value |
|---|---|
| Recorded EXE | `evidence/artifacts/i03a/recorded-opensequoia.exe` (copied from `opensequoia-crew/controller/target/i03a/release/opensequoia.exe`) |
| Recorded bytes / SHA256 | 1,043,456 / `b8ae67c3b6839c1be1751bb3eb49088bf9999491b38284b58eb309e74cbc7499` |
| Canonical rebuild 1 / SHA256 | `evidence/artifacts/i03a/rebuilt-default1-opensequoia.exe` / `df0f8b9c1a55ac7cbb13db9abffcfe435b78d2d17e1f243bb3a6c9709f32e23c` |
| Canonical rebuild 2 / SHA256 | `evidence/artifacts/i03a/rebuilt-default2-opensequoia.exe` / `ed265d2f30cc9e2a884aa6f8a7fd5c679660baaa07ca7fed5074bfbb679ead1e` |
| `/Brepro` rebuilds (identical) | `evidence/artifacts/i03a/rebuilt-brepro{1,2}-opensequoia.exe` / `165af357529758fd51a54df676c629a60331637f3ceef2d436c467b6c3572d26` |
| Source | `54289fd358547870b6fce3b9040a959dfad4efc5` |
| Toolchain | `rustc 1.98.1 (48a229cea 2026-09-01)` |

Rebuild command: `cargo build --release --locked --target-dir <fresh> -p opensequoia`, run in a detached worktree of `54289fd`. `/Brepro` runs add `RUSTFLAGS='-C control-flow-guard=yes -C link-arg=/Brepro'`.

## Observed

1. **Recorded vs canonical rebuild 1:** 24 differing bytes, all inside parsed identity fields: COFF `TimeDateStamp` (2 of its 4 bytes differ), three debug-directory `TimeDateStamp` fields (2 bytes each), and the CodeView RSDS GUID (16 bytes). Masking only those fields makes both files hash to `b1ba0a98b4ccc2de084f03a56c4b82a30c37420a9a40c08b00774250ee75cc10`.
2. **Section payloads:** `.text`, `.data`, `.pdata`, `.rsrc` and `.reloc` are byte-identical. `.rdata` differs only in the identity fields and is equal after masking. There is no code-byte difference between the recorded artifact and the clean rebuild.
3. **Canonical build is not deterministic:** rebuild 1 vs rebuild 2 differs in 20 bytes (COFF timestamp, debug timestamps, PDB GUID). Masked hashes are equal, but `cargo build` alone must not be called bit-reproducible.
4. **`/Brepro` is cross-run deterministic:** two fresh builds are byte-identical and match the earlier `/Brepro` hash exactly. It adds a 28-byte type-16 Repro debug-directory entry (debug directory 84 → 112 bytes), which shifts `.rdata` and the RVAs that point into it: 22,570 bytes differ from the canonical build (4 `.text`, 21,098 `.rdata`, 1,461 `.pdata`, 7 headers). Masked equality with the recorded EXE is **false**.
5. **Runtime:** recorded and canonical rebuild print byte-identical `--inspect` output for `fixtures/guide.html` (`dca111c5...`) and `fixtures/welcome.html` (`11bc5829...`). Both binaries also pass native windowed `--smoke-test` (`WINDOW_SMOKE_OK paints=1 drawn_text_runs=105 child_rendered=1`); the current reviewed build passes `--smoke-test` and `--app-smoke` (`APP_SMOKE_OK navigation=1 back=1 forward=1 tabs=1 close_reset=1 native_controls=1 child_rendered=1 paints=10`).

## What this proves

- The recorded artifact is content-equivalent to a clean canonical rebuild: code and data sections match, and only PE build identity differs.
- The canonical command is not bit-for-bit reproducible because timestamps and the PDB GUID vary.
- `/Brepro` alone is deterministic across runs, but it changes the layout; adopting it requires a new baseline, not a claim that it reproduces the recorded bytes.

## Limits

- The recorded EXE predates the final `worker.rs` commit; `root4-build.json` records the committed file as byte-identical to the build tree, but this check cannot recover historical dirty bytes.
- No Authenticode signature, malware review or remote-content test was performed. Windowed local smoke passed, but it is not site isolation, hostile-content or release-trust evidence.
- A partial `cargo clean -p` rebuild after changing linker flags produced a 1,039,360-byte binary with different code; checks must use fresh target directories.
- The independent review was static and single-model; it caught the normalization defect but is not a security audit.

## Next executable actions

1. Decide whether `/Brepro` (plus CFG) becomes the canonical release recipe; if yes, re-baseline the artifact and require two clean `/Brepro` hashes to match. Do not call the canonical build bit-reproducible before that.
2. Add Authenticode signing/verification, malware review and a packaged (ZIP-installed) windowed smoke as separate S10 gates.
3. Keep the recorded and rebuilt artifacts and this report as evidence; do not claim signed or universally reproducible release status.
