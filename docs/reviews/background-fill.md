# Block background fill and IPC v2 review

Status: feature verified at development scope; independently reviewed in a constrained excerpt-only round. Not a security audit, CSS conformance result or release acceptance.
Date: 2026-11-24. Scope: uncommitted block `background-color` support: `FillRect`, layout fill spans, IPC wire version 2, native GDI painting, tests and docs.

## Change

- Engine emits a `DisplayItem::Fill(FillRect)` for an explicit background colour on a block box, covering content + padding + border (not margin), before descendant text. Zero-area fills are removed at block exit.
- IPC version 2 tags items: 1 = text (29 fixed bytes + UTF-8 length + text), 2 = fill (20 bytes). Both decode paths validate geometry, count, total text and exact payload consumption. Version 1 frames are intentionally rejected.
- The native paint loop draws fills in item order through a temporary solid brush and then draws text; both use signed i64 CSS-to-DPI vertical offsets and checked narrowing.
- `style` is now a recognized (non-rendered) element, so a normal `<style>` sheet no longer produces a spurious unsupported-element diagnostic.

## Independent review

Reviewer: app-owned subagent, provider `deepai`, model `deepai/claude-haiku-5-5`, thinking high. Review was static and read-only.

- Two attempts to review the complete 40 KiB diff failed before producing a verdict with `context_length_exceeded`. They are not counted as review evidence.
- A compact excerpt review returned **REJECT**. Its findings and adjudication:
  - **P1 payload length ignored text bytes** - false. `validate` adds `TEXT_ITEM_BYTES + run.text.len()` per text item with `checked_add`; fills add 20. The reviewer was given prose rather than the actual encode loop.
  - **P2 `MIN_ITEM_BYTES` might be 29** - false. The smallest record is a 20-byte fill; `MIN_ITEM_BYTES = 20` is tested by fill-only frames.
  - **P2 scroll overhang clamped to zero** - the claim that `i32::saturating_sub` clamps at zero was incorrect, but the code was changed to compute `i64::from(rect.y) - i64::from(scroll)` and then scale, removing any ambiguity and guaranteeing negative overhang before GDI clipping. Also applied to text.
  - **P2 GDI coordinate overflow** - false with the new `scaled_offset` (`checked_mul`, i64 division, `i32::try_from`); rectangle additions already saturate.
  - **P2 layout exit reads a fill when none was pushed** - false. `exit.fill` is `Option<FillSpan>` and is only `Some` when a fill was pushed; a missing span does nothing.
  - **P3 `line_start` not adjusted on fill removal** - addressed defensively: removal decrements `line_start` when it lies after the removed index. No reachable reproducer was found for the claimed text-skip, but the invariant is now explicit.
  - **P3 brush lifetime** - false. `FillRect` and `DeleteObject` both run on the same path; no `?` appears between them. The brush is never selected into the DC.
- A final excerpt-only check returned **APPROVE** after the i64 scaling change. The reviewer explicitly stated it did not read the repository files. This is a narrow review of the supplied excerpts, not full-file review or a security audit.

## Evidence

- `evidence/verification/fill-final2-tests.log`: 94 passed, 0 failed, 0 ignored (42 app, 2 dat, 18 CSS, 4 tree-inspection fixture, 9 static document, 8 tree inspection, 11 IPC).
- `evidence/verification/fill-final2-clippy.log`: Clippy clean with `-D warnings`; formatting checked by the gate.
- `evidence/verification/fill-final2-verification.json` plus `fill-final2-*` logs: `DEVELOPMENT_GATES_OK`, `PROCESS_SMOKE_OK`, `WINDOW_SMOKE_OK paints=1 drawn_text_runs=105 child_rendered=1`, `APP_SMOKE_OK ... paints=10`.
- `evidence/verification/fill-final2-background-inspect.log`: fixture has 25 nodes, 35 text runs, 0 unsupported elements/CSS, one limited-layout feature diagnostic.
- `evidence/verification/fill-final2-background-smoke.log`: `WINDOW_SMOKE_OK paints=1 drawn_text_runs=34 child_rendered=1`. This executes the GDI fill path; it does not compare pixels.
- Pinned tree acceptance (`evidence/verification/tree3-final.log`): `S04A_ACCEPTANCE_OK`, 3515 pass / 4 unsupported / 0 fail / 0 skip / 0 timeout / 0 harness error; display changes do not alter parsing.

## Limits and non-claims

- Only explicit block background colours are painted. Inline backgrounds, border painting, canvas/root propagation, gradients, images, alpha, `rgb()` and the `background` shorthand remain unsupported.
- Fill geometry is an integer CSS-pixel box; text metrics remain estimates and there is no shaping, selection, CSS conformance, pixel or website compatibility claim.
- Wire version 2 is incompatible with historical version 1 peers by design; the app and worker are built together. There is no authenticated peer, origin or capability check.
- No new `unsafe` was introduced. GDI painting is existing documented Win32 usage; the review above does not replace the outstanding signing, sandbox and hostile-content work.
