# CSS subset (static documents)

Status: implemented and tested increment; **not** CSS conformance and not a compatibility claim.
Owner: root integration. Source: `crates/engine/src/css.rs` + styled flow in `crates/engine/src/layout.rs`.

## Supported surface

- Sources: `<style>` element text and `style="..."` attributes. External stylesheets are not fetched. CSS comments are stripped in both; supported value keywords are ASCII case-insensitive.
- Selectors: type, `.class`, `#id`, `type.class`, `type#id`, and comma-separated lists.
  No combinators, universal/attribute/pseudo selectors, or `!important`.
- Cascade: inline style specificity 1000; otherwise `#id` 100, `.class` 10, type 1; ties use later source order.
- Declarations:
  - `color`, `background-color`: `#rgb`, `#rrggbb`, or a 16-name subset (aqua, black, blue, fuchsia, gray, green, lime, maroon, navy, olive, purple, red, silver, teal, white, yellow). No `rgb()` yet.
  - `font-size`: integer px, 1-256 (the IPC v2 display bound; larger values are unsupported).
  - `font-weight`: `normal` or `bold`; `text-align`: `left`, `center`, `right`.
  - `margin`/`padding` (+ `-top/right/bottom/left`): 1-4 integer px values, shorthand expansion.
  - `border`, `border-width`, `border-style` (`solid`/`none`), `border-color`: width affects layout and a solid border is painted as four axis-aligned fill records after its content. A one-value `border: Npx` means a solid width of N (a documented subset deviation from CSS, where the style would default to `none`); `border:none` resets both width and style.
  - `display`: `block`, `inline`, `none`. `none` suppresses the subtree.
- Inheritance: text colour, size, weight and alignment inherit; background colour and box properties do not.
- Defaults: `html`/`body` and `p`/`div`/headings are block boxes; body is 16px and colour `#1c2923`; headings are bold with absolute default sizes (32/28/24/20/18/16px, not em-relative); `<b>`/`<strong>` bold.

## Layout behaviour

Normal flow only: block/inline boxes, word wrap, text alignment within the content box, and
margin/padding/border insets. An explicit `background-color` on a block box becomes an IPC v2 fill
record covering its content, padding and border box, painted before its descendants; it does not
propagate from `body` to the canvas, and inline backgrounds are not painted. A solid block border
becomes four fill records (top, bottom, left, right) painted after the block text; corners and short
edges can overlap, and only axis-aligned solid borders are represented. There is no float, flex,
grid, positioning, collapsing margins, or bidirectional/complex shaping.

## Bounds and failure policy

- At most 4096 rules **shared by all `<style>` elements**, not 4096 per sheet; later sheets are skipped and counted. A selector longer than 256 bytes is skipped and counted.
- Lengths above 100,000px, non-px units, negative values, malformed shorthands and unknown
  properties/values are skipped and counted, never fatal.
- `unsupported_css_declarations()` and `Diagnostic::UnsupportedCss` expose the count; the app's
  `--inspect` summary prints at least one diagnostic for a nonzero count.
- Layout remains bounded by `MAX_HEIGHT`, `MAX_RUNS` and the existing node cap.

## Verification

- `crates/engine/tests/css_subset.rs`: 20 tests covering specificity, source order, lists, unsupported
  accounting, supported-set accounting, font/size/weight/colour/alignment, spacing, borders, display,
  inheritance, unstyled default drift, the 1-256 IPC font bound, the shared rule budget, value case/comments,
  `border:none`, block fill geometry, border edge order/geometry, and empty-block fill suppression; a unit
  test covers border colour defaulting to the element's own colour.
- `crates/app/src/worker.rs` tests: a 256px styled document and block background/border fills survive
  `Document::parse` -> layout -> IPC encode/decode through the real worker boundary.
- `fixtures/background.html` is a native-paint fixture: current-build `--inspect` reports no
  unsupported CSS and `--smoke-test fixtures/background.html` exercises the GDI background and block
  border fill paths before automatic close. This proves the fill calls succeed, not pixel equality or
  full CSS rendering.
- `cargo test --workspace --locked`: 96 passed, zero failed/ignored (20 CSS-subset tests).
- The pinned tree-construction acceptance harness passes
  (3515 pass / 4 unsupported, no harness errors) and is unaffected by display-list decoration.

## Non-claims

No CSS conformance, browser compatibility, external CSS, custom properties, `calc`, `em`/`rem`/`%`,
selector combinators, pseudo-classes, media queries, animations/transitions, inline backgrounds, canvas
background propagation, border-radius/dashed/dotted/double styles, collapsed borders, or fonts/shaping.
Solid block borders are represented only as four fill records; this is not border-style or painted-fidelity
conformance. Values outside the subset are visible as counted unsupported declarations.
