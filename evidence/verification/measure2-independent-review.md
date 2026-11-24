# M02 independent measurement review

Reviewer: `measurement-review`, leaf 1.1.3, independent from `artifact_release`

Lane: `crew/m02` at base `2027b2274f975f68f583a67aaed16f6b9ec89b38`

Verdict: **PASS_WITH_EXPLICIT_LIMITS** for reconciliation of the recorded exploratory evidence. This is not formal READY, a production performance gate, or a Brave comparison. The GUI workload was not rerun.

## Findings

No correctness defect was found in the raw arithmetic, recorded before/after identities, or current fingerprints. Two evidence limitations deserve explicit retention:

1. **Renderer coverage is absent from the samples.** Every one of the 140 samples contains only the GUI root. No renderer child was observed at the 250 ms interval, so the data cannot support renderer memory, exited-child CPU, or true whole-process-tree peak claims.
2. **Environment capture is incomplete for rigorous comparisons.** The raw report records OS/hardware/Python/psutil/power/RAM but not viewport, DPI, fonts, or a background-process inventory requested by `BENCHMARKS.md`. This is a future measurement-protocol gap, not a reason to alter immutable raw evidence.

The first isolated oracle run also exposed a migration defect in the new oracle only: it resolved the raw report's legacy absolute sampler path against the crew worktree and exited 1 before any process control. The oracle now distinguishes legacy read-only evidence from worktree-owned review code. The sampler, focused checks, and raw report were not edited.

## Source and evidence identity

| Input | Bytes | SHA256 | Result |
|---|---:|---|---|
| [app2-idle.json](evidence/benchmarks/app2-idle.json) | 39687 | `ac5e2799ab5a8c91a895cccaeb81a2c083a5f69a101a49a8580fb5b2a16c004d` | Immutable before/after controls |
| Immutable GUI EXE at legacy absolute path | 946176 | `d80778d7dbaa7f3a520b33e88ab3cb8346cce1c8c31749cba19653ede0511f3e` | Current and before/after records match build record |
| [welcome.html](fixtures/welcome.html) at legacy absolute path | 1358 | `68cedd95f6f8125e334f748f1090cfb29f27fa7affd37e86214a90469c2294ea` | Current and before/after records match |
| [measure_app_idle.py](scripts/measure_app_idle.py) | 9825 | `80f26bbae8e0ca5486cc8f8ec724eb590209cf7f0eb355d39b2f9a865832df8b` | Legacy, worktree, and raw record match |
| [measure_idle.py](scripts/measure_idle.py) | 6133 | `e993b69a87d2a97b76f1674afd5ceff9a8a566462f3fa8d1ee5fa87daa6739c9` | Legacy and raw record match |
| [measure2-checks.py](evidence/verification/measure2-checks.py) | 2017 | `074c97b5fcc36758ef299ac4b53940f035f9a1c4760da86cde24b6923da30cf1` | Migrated byte-identical |
| [measure2-acceptance.py](evidence/verification/measure2-acceptance.py) | 28097 | `b5c1b7bebd5789933faf499eb458fc89bb507304910f234c465ee1cf67c2f5ec` | New independent oracle |

The recorded application source checkpoint is `83451aa1a93f7c85b730dd30940975bd7abf6aaa`, and Git resolves that full revision to a commit. Its association with the immutable EXE is historical identity, **not** a reproducible compiler-build proof. Development source has advanced. The migration audit is [migration-audit.json](target/m02/migration-audit.json); all four copied files are byte-identical to legacy.

## Observed raw facts

- Seven runs contain exactly 20 samples each: **140 samples total**.
- Sampled process-count set: **`[1]`**. Each run's maximum sampled process count is 1.
- Final samples occurred **4768.742600001133–4987.052600001334 ms** after Popen initiation. These are elapsed sample observations, not startup or readiness metrics.
- Recorded environment: Windows 11 build 26200, AMD64, Intel64 Family 6 Model 154 Stepping 4, 12 logical/10 physical CPUs, 16716918784 bytes installed RAM, Python 3.13.5, psutil 7.1.2, and Power saver.
- Available memory changed from 7322234880 to 7370428416 bytes. Every other recorded environment field is equal before/after. Host background work and cache conditions were not controlled.
- All seven recorded PID/create-time identities are no longer running. No recorded identity remains alive with the same birth time.
- The legacy Brave harness and data are tracked and have empty worktree diffs, empty cached diffs, and empty path-scoped porcelain status. [brave-idle.json](evidence/benchmarks/brave-idle.json) SHA256 is `ecbbdcc1873b97357ce4a2d920fd86f65b0ab220c021ddc4e4cca71ffc3457c7`. No Brave statistics were recomputed for an equivalence comparison.

## Independently recomputed statistics

The oracle uses its own sorted-values median implementation, recomputes per-run final/peak/process-count fields from raw samples, and compares every published distribution exactly.

| Metric | Median | Minimum | Maximum |
|---|---:|---:|---:|
| Final private commit bytes | 1421312 | 1409024 | 1421312 |
| Final working-set bytes | 8282112 | 8273920 | 8310784 |
| Final sampled process count | 1 | 1 | 1 |
| Sampled peak private commit bytes | 1421312 | 1409024 | 1421312 |
| Sampled peak working-set bytes | 8282112 | 8273920 | 8310784 |

Final private median is 1.35546875 MiB, with range 1.34375–1.35546875 MiB. Final working-set median is 7.8984375 MiB, with range 7.890625–7.92578125 MiB. Final and sampled-peak distributions coincide in this dataset. The peaks remain maxima of root-only samples, **not true peaks**.

## Verification actually run

Commands ran from the isolated `crew/m02` worktree with `PYTHONDONTWRITEBYTECODE=1` for Python invocations, preventing `__pycache__` writes outside owned paths.

| Command / control | Result |
|---|---|
| `python evidence/verification/measure2-checks.py` | Exit 0; **2/2 tests passed**, `OK` |
| `python evidence/verification/measure2-acceptance.py --details` | Exit 0; **197/197 assertions**, `M02_ACCEPTANCE_OK` |
| `python evidence/verification/measure2-acceptance.py` | Exit 0; exact required success token `M02_ACCEPTANCE_OK` |
| `python target/m02/refute-acceptance.py` | Exit 0; **12/12 corrupted cases rejected** for their intended assertions, `M02_REFUTATION_OK` |
| Current-source wrong-hash control | Exit 2; explicit hash error; **0 run/Popen calls** because launches were stubbed to forbid any GUI rerun |
| Real Python-child cleanup control | Wrong birth identity preserved; matching launch-owned root killed and reaped |
| Legacy Git baseline checks inside oracle | Both files tracked; worktree diff, cached diff, and porcelain status empty |
| Ruff lint/format commands | **Unavailable**: `ruff` is not recognized on this lane's PATH; no current lint pass claimed |

Acceptance assertion categories: baseline Git 8, cleanup/rejection 8, derived statistics 34, environment 28, limitations 12, raw metadata 9, raw structure 81, and source identity 17. The 197 total includes a summary-mutation positive control and raw-hash checks before/after dynamic controls.

The 12 refutations cover a missing sample, final/last-sample mismatch, false private peak, false summary median, unrecorded child count, executable after-hash drift, sampler hash drift, missing environment field, removed limitation, repeated elapsed time, invalid root birth, and whole-file raw mutation. See [refutation-results.json](target/m02/refutation-results.json). The real raw SHA256 remained `ac5e2799ab5a8c91a895cccaeb81a2c083a5f69a101a49a8580fb5b2a16c004d`.

## Test value card

`Value: protects=M02 raw arithmetic, source identity, baseline integrity, and launch-owned cleanup; fails_when=a sample, digest, summary, PID birth identity, rejection, or Git baseline changes; why_new=existing focused checks omit all-sample, environment, hash, wrong-hash, and mutation reconciliation; seam=none in production, with wrong-hash launches stubbed only in the test process; no_claim=no GUI rerun, paint/readiness, renderer/true peak, Brave equivalence, startup/navigation, or production-budget verdict.`

## Claims and limits

### Observed

- The recorded run/sample counts, process counts, environment fields, and PID identities above are present in raw evidence.
- Current EXE, fixture, and sampler fingerprints match their recorded identities; executable and fixture before/after records are equal.
- Current-source rejection and real Python-child cleanup controls passed without launching the GUI.

### Derived

- All five published summaries and every per-run final/peak/process-count field recompute exactly from raw samples.
- Zero renderer child identities were recorded at 250 ms. This does not prove that a renderer never ran.
- Final elapsed observations range from about 4.769 to 4.987 seconds after Popen initiation.

### Not established

- Renderer lifetime/resource use, true all-process-tree peaks, or total exited-child CPU.
- First meaningful paint, page readiness, navigation, interaction, frame pacing, or startup-to-paint timing.
- Cold startup, controlled cache state, energy use, active/multitab/recovery behavior, or production budgets.
- Equivalent Brave functionality, a comparative advantage, reduction ratios, or fastest/lightest claims.
- A reproducible build from the historical source checkpoint or performance of later development source.

Working-set sums may count shared pages more than once. CPU values cover only extant sampled processes. Hidden-window process survival and bounded headless fixture inspection do not prove paint readiness. Creation-time checks and the Popen handle constrain cleanup to owned identities, but children never observed cannot be included in cleanup evidence. This review reconciled evidence and non-GUI controls; it did not independently reproduce the GUI workload.

## Root actions

1. Preserve [app2-idle.json](evidence/benchmarks/app2-idle.json), [measure_app_idle.py](scripts/measure_app_idle.py), [measure2-checks.py](evidence/verification/measure2-checks.py), and the Brave baseline as immutable inputs. Integrate this only as a local crew candidate, not formal READY.
2. If root records these statistics in `BENCHMARKS.md` or `TASKS.md`, retain the renderer-sampling, environment-capture, paint-readiness, and non-equivalence limits.
3. Use a future new workload for explicit paint/navigation readiness, high-resolution renderer-lifetime instrumentation, viewport/DPI/font/background capture, and full-process-tree active/peak/recovery metrics before any comparison.
4. Keep formal gstack receipt/READY/ship gates open: helper runtimes and remote policy are unavailable. No unavailable framework result is claimed here.
