# Review: I03a reproducibility evidence

Reviewer: app-owned Claude Haiku 5.5 subagent (DeepAI provider, high thinking), task
`node:delegated-task:command%3Amcp%3Abcf0fc16-627d-4141-8990-7c171cba1e65%3Adelegate-task%3Ai03a-review-haiku-1`.
Scope: the schema-1 handoff and `i03a-repro.json` text. Verdict: **REJECT**.

## Findings and disposition

| Sev | Finding | Disposition |
|---|---|---|
| High | `brepro.recorded_offsets` listed 22,570 bytes while the prose claimed exactly 24 timestamp bytes. | **Confirmed and corrected.** Schema 2 reports 24 recorded-vs-default bytes and 22,570 `/Brepro`-vs-default bytes separately; the contradiction is removed. |
| High | The 24-byte timestamp explanation did not fit the offsets. | **Confirmed.** 16 of the 24 bytes are the CodeView PDB GUID, not timestamps; the rest are COFF/debug timestamps. Prose now says so. |
| High | Normalizing all differing bytes made equality tautological. | **Confirmed.** Schema 2 masks only PE identity fields located by parsing; masked hashes are `b1ba0a98...`. |
| Medium | The CFG/normalized story was inconsistent. | **Resolved by correction.** The old normalized hashes were tautological; `/Brepro` changes layout and is not claimed equivalent. |
| Medium | Recorded EXE predates the final `worker.rs`; provenance was overstated. | **Kept as a limit.** `.text` is byte-identical to the clean rebuild, but historical dirty bytes are still not recoverable. |
| Low | Runtime evidence was one fixture. | **Expanded** to `guide.html` and `welcome.html`, both byte-identical output. |

## New evidence after correction

- Two clean canonical builds are **not** byte-identical (20 bytes differ), so the canonical command is not called bit-reproducible.
- Recorded vs canonical rebuild: code sections identical, 24 identity bytes differ, masked equality `b1ba0a98...`.
- `/Brepro`: two builds identical (`165af357...`), debug directory 84 -> 112 bytes, 22,570 bytes differ from canonical, masked equality false.

Residual risk: the review was static, single-model and text-only; it is not a security audit.
