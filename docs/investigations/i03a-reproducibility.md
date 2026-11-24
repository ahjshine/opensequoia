# I03a: artifact reproducibility review

Status: **content-equivalent modulo PE build identity**, corrected after independent review; canonical builds are not bit-reproducible, and `/Brepro` is deterministic only as a layout-shifted recipe. Not signed.

See `evidence/verification/i03a-repro-handoff.md` for commands, hashes and limits and `evidence/verification/i03a-repro.json` (schema 2) for the machine-readable analysis. Committed copies of the five binaries are under `evidence/artifacts/i03a/`.

- Recorded: 1,043,456 bytes, SHA256 `b8ae67c3...`.
- Recorded vs clean canonical rebuild: 24 differing bytes, all COFF/debug timestamps and the CodeView GUID; `.text` and all non-`.rdata` sections byte-identical; masking only parsed identity fields gives `b1ba0a98...` on both.
- Two clean canonical builds differ by 20 bytes, so the canonical command is not bit-reproducible.
- Two `/Brepro` builds are byte-identical (`165af357...`) but differ from the canonical build in 22,570 bytes because a 28-byte Repro debug entry shifts `.rdata`; no normalized match is claimed.
- `--inspect` output matches for `guide.html` and `welcome.html`.

Correction: schema 1 zeroed every differing byte independently, making its normalized equality tautological; an app-owned Haiku 5.5 review returned REJECT and schema 2 replaces that check.

Limits: historical dirty-tree provenance is inferred from `root4-build.json`, not re-derived; the artifact is unsigned and not malware-reviewed; no remote-content test; the canonical release command does not include `/Brepro`. Windowed local `--smoke-test` passed for recorded, rebuilt and current builds, and current `--app-smoke` passed; these are local GUI checks, not site isolation.
