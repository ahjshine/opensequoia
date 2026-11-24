# S04a HTML tree-construction inspection and corpus

Status: local development evidence for the bounded OpenSequoia engine increment. This is not a browser-compatibility, production-security, coverage, or performance claim.

## Scope and implementation

`Document::parse` still parses a complete HTML document with scripting disabled. `Document::parse_with_options` additionally selects the parser scripting flag or a bounded HTML/SVG/MathML fragment context. Both paths use the real html5ever `TreeSink`; no script is executed and no network request is made.

`Document::tree_dump` performs an iterative, deterministic traversal of the flat arena. It preserves doctypes, comments, text, namespaced elements/attributes, and detached template contents. Attribute names are sorted by UTF-16 code units, as required by the pinned tree format. This is diagnostic tree inspection, not HTML serialization, a live DOM API, CSS layout, or browser DOM behavior.

Bounds are enforced rather than returning a successful partial result:

- 262,144 UTF-8 input bytes and 32,768 arena nodes;
- 4 MiB of tree-dump output, with checked output-length arithmetic;
- iterative traversal/drop and bounded sibling/event walks;
- at most 1 MiB per `.dat` file and a 5,000 ms deadline per case subprocess;
- a 900 s outer acceptance deadline and Cargo output only under `target/tree3`.

A bounded engine rejection or a tree mismatch is a `fail`. It is never converted to `skip` or `unsupported`.

## Exact source identity

| Input | Observed pin |
|---|---|
| WPT tree-construction suite | `31d1feb70e2d02f25ae26254caba19c8de0b56e0` from `rcdom/data/test/wpt-commit.txt` |
| Inspected html5ever adapter checkout | `492f52a02fd07c2321e59ccff90038e26b9c90bc`, runner `rcdom/tests/html-tree-builder.rs` |
| Actual locked parser | html5ever `0.40.1`, registry VCS `d7232d746d3112f08926d169591576bd12fe2fdf` |
| Parser archive SHA-256 | `456a1a377e608e555d22ddab27ac0114bc7a7b4199078108e34c2aeae6c9b130`, matching `Cargo.lock` |
| Suite inventory | 63 suite files plus 5 upstream runner/revision inputs, 68 total, independently hashed in `evidence/verification/tree3-suite-hashes.json` |
| Inventory SHA-256 | `563bfe0fd1c9f450f0395d621127434dd685ecbabe514e34547f5069243ef465` |
| Locked manifest SHA-256 | `e801e3ec0b12e1d134834a119102c4ac270d328818683c5c7cdb4d11b5852707` |
| Engine manifest SHA-256 | `a50e98201067c464496bcaa4643a55ee081ab1e2e2433410fe5ddd4f040b8173` |

The original inputs stay read-only in the legacy source checkout. The acceptance script derives that checkout as `../../opensequoia` relative to the isolated lane root. Git's Windows CRLF working-tree conversion is handled like upstream `BufRead::lines`: CRLF is a line ending, while an isolated or final-unterminated carriage return remains data. The hashes cover the actual working-tree bytes, so line-ending changes are detected rather than silently ignored.

The upstream `data/test/ignore` file is hashed and inspected for provenance only. It is **not applied** by this adapter.

## Outcome contract

Every source case retains its filename, zero-based source index, fragment context, and scripting mode. Unflagged cases run in both modes; `#script-on` and `#script-off` select their respective single mode.

| Outcome | Meaning |
|---|---|
| `pass` | Actual canonical tree exactly equals the source `#document` tree |
| `fail` | Tree mismatch or a bounded engine rejection; both remain visible product failures |
| `unsupported` | Only upstream `scripted_*.dat` cases requiring JavaScript execution during parsing |
| `skip` | Available accounting category; no case is skipped by this run and upstream ignores are not imported |
| `timeout` | Case subprocess exceeded its recorded deadline |
| `crash` | Child terminated unsuccessfully outside the adapter's explicit harness-error exit |
| `harness_error` | Spawn, I/O, cleanup, reply, source-parse, or other adapter malfunction |

The acceptance oracle independently parses the source census, rebuilds the expected identity/mode set, validates every JSONL record and summary, and rejects missing, duplicate, or unexpected cases. It requires zero `skip`, `timeout`, `crash`, and `harness_error` outcomes. Genuine `fail` outcomes are permitted and printed. Eight negative controls prove that missing cases, duplicate cases, a wrong source revision, harness errors, timeouts, unexpected unsupported outcomes, hidden skips, and summary mismatches cannot produce `S04A_ACCEPTANCE_OK`.

## Reproduce

Run from the isolated lane repository root. The normal acceptance command does not modify the source pin.

```powershell
cargo fmt -p opensequoia-engine -- --check
cargo clippy --target-dir target/tree3 -p opensequoia-engine --all-targets --locked -- -D warnings
cargo test --target-dir target/tree3 -p opensequoia-engine --locked
python evidence/verification/tree3-acceptance.py
```

The acceptance command builds the example, runs all applicable outcomes, validates the report, and prints `S04A_ACCEPTANCE_OK` only after every assertion passes. Its successful output currently includes the four engine failures below.

To run the adapter directly, resolve the legacy suite and use owned report/scratch paths. Exit 1 is expected while genuine `fail` or `unsupported` outcomes exist; exit 2 is an operational harness error.

```powershell
$legacy = (Resolve-Path "$PWD/../../opensequoia").Path
$suite = Join-Path $legacy 'evidence/components/html5ever/rcdom/wpt-tree-construction-tests'
cargo run --target-dir target/tree3 -p opensequoia-engine --locked --example tree_construction -- `
  --suite $suite `
  --suite-revision 31d1feb70e2d02f25ae26254caba19c8de0b56e0 `
  --report target/tree3/manual-report.jsonl `
  --scratch-dir target/tree3/case-runs `
  --timeout-ms 5000
```

Reproduce one full actual tree (the result begins with `OK` or `ERROR`):

```powershell
target/tree3/debug/examples/tree_construction.exe --case `
  "$suite/webkit02.dat" 44 off target/tree3/webkit02-44-off.txt
```

Only after an intentional suite/revision change, independently review the new inputs and regenerate the pin. This maintenance command does not print the acceptance token:

```powershell
python evidence/verification/tree3-acceptance.py --write-inventory
git diff -- evidence/verification/tree3-suite-hashes.json
python evidence/verification/tree3-acceptance.py
```

## Observed corpus result

The current pinned corpus has 59 `.dat` files and 1,779 source cases. Scripting expansion produces 3,519 outcomes.

| pass | fail | unsupported | skip | timeout | crash | harness_error |
|---:|---:|---:|---:|---:|---:|---:|
| 3,511 | 4 | 4 | 0 | 0 | 0 | 0 |

The unsupported identities are `scripted_adoption01.dat#0@on`, `scripted_ark.dat#0@on`, `scripted_webkit01.dat#0@on`, and `scripted_webkit01.dat#1@on`.

The four failures are `webkit02.dat#44@off`, `webkit02.dat#44@on`, `webkit02.dat#45@off`, and `webkit02.dat#45@on`. Their expected trees contain cloned option content under `selectedcontent`; the actual trees do not. This is consistent with the unimplemented html5ever `maybe_clone_an_option_into_selectedcontent` sink hook. The same two source indices appear in upstream's ignore file, but this run deliberately keeps all four as `fail`. Exact byte-99 mismatch excerpts are in `evidence/verification/tree3-report.jsonl`.

## Limits and claims

- **Observed:** the commands and counts above were run on the isolated lane sources; unit/integration tests cover retention, fragments, namespaces, malformed input, Unicode ordering, deep traversal, and resource rejection.
- **Derived:** the outcome total and summary are independently recomputed from source identities and report rows. The selectedcontent explanation follows from the expected/actual trees and the upstream sink hook/ignore entry.
- **Not established:** parse-error text (`#errors`/`#new-errors`) conformance, JavaScript execution, live DOM mutation, contextual fragment ancestors/form owners, full WPT coverage, modern website compatibility, performance distributions, memory ceilings beyond the explicit allocation/output bounds, or production sandboxing.
- Per-case stderr is not retained on a healthy run. A crash or harness error blocks acceptance and must be reproduced with `--case` for investigation rather than hidden. No such outcome occurred in the accepted report.
