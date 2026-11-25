# Review: bounded CSS subset

Reviewer: app-owned Claude Haiku 5.5 subagent (DeepAI provider, high thinking), task
`node:delegated-task:command%3Amcp%3Abcf0fc16-627d-4141-8990-7c171cba1e65%3Adelegate-task%3Acss-review-haiku-inline-1`.
Scope: full inline text of `crates/engine/src/css.rs`; tests and layout were checked by root after the review.
Verdict: **changes required**; the P1 and actionable P2/P3 findings below were fixed or explicitly documented before integration.

## Findings and disposition

| Sev | Finding | Disposition |
|---|---|---|
| P1 | The 4096-rule cap was per `<style>` element; many sheets multiplied the rules scanned for every node. | **Fixed.** One shared budget across all sheets; later sheets counted unsupported. Regression test `stylesheet_rule_budget_is_shared_across_all_style_elements` (two 4096-rule sheets -> one unsupported, page still renders). |
| P2 | `border:1px` implied solid, where CSS defaults the style to none. | **Documented subset deviation** (a one-value border is deliberately a solid width; existing layout test depends on it). `border:none` now resets width and style; test `border_none_resets_width_without_becoming_unsupported`. Multi-token shorthand remains unsupported and counted. |
| P2 | Border colour defaulted to the inherited colour rather than the element's own `color`. | **Fixed** in `Cascade::resolve`; unit test `border_color_defaults_to_the_elements_own_color`. Border painting was added later as four solid edge fills. |
| P2 | Non-element nodes copied the parent's full style, leaking box properties. | **Fixed** to `ComputedStyle::inherited`, which keeps text properties and resets box values. |
| P2 | `<style>` text might be laid out. | **Not a defect:** `layout.rs` skips `head`, `script`, `style` and `template` before style lookup; verified by inspection and existing tests. |
| P3 | CSS keywords/colour names were matched case-sensitively. | **Fixed** by ASCII-lowercasing values; test `values_are_case_insensitive_and_inline_comments_are_ignored`. |
| P3 | Inline styles were not comment-stripped. | **Fixed** by stripping comments in `parse_declarations`; same test. |
| P3 | `style_texts` can drop oversized text parts silently. | Not reachable under the 256 KiB document cap: the sum of style text is bounded by the input. Limit remains documented. |
| P3 | Heading defaults are absolute (32/28/...) rather than em-relative. | Documented in `docs/testing/css-subset.md` as a subset deviation; `<div style="font-size:40px"><h1>` therefore yields 32px. |
| P3 | Border shorthand does not reset all longhands. | Accepted for the width-only subset and documented; no colour painting exists. |

## Verification after fixes

- `cargo test --workspace --locked`: 90 tests at this review, zero failed/ignored. Current count is 94 after block background fills (`docs/reviews/background-fill.md`).
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: clean.
- `cargo fmt --all -- --check`: clean.
- `python evidence/verification/tree3-acceptance.py`: 3515 pass, 0 fail, 4 scripted unsupported.

## Residual risks

Rule matching is still linear in the shared 4096 rules per element; a near-cap document can do bounded but noticeable work. There is no selector index, CSS conformance, external CSS, `!important`, combinators, border-style/radius fidelity, or border painting beyond four solid edge fills. The review was static and single-model; it does not replace a security audit.
