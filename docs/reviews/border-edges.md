# Solid block border edges review

Status: development-verified increment with a narrow snippet review. Not a security audit, CSS border-conformance result or release acceptance.
Date: 2026-11-24. Scope: uncommitted solid block border painting in `crates/engine/src/layout.rs`, its engine/worker tests, the native fixture and user-facing limits.

## Change

- A block with a positive effective `border-width` and `border-style: solid` records a `BorderSpan` at entry. At block exit, after its text, layout appends four `FillRect` records: top, bottom, left and right edges.
- Edges use the same outer border box as the background fill (content + padding + border, excluding margin). Thickness is clamped to the edge width and available border-box height so tiny boxes cannot produce a fill wider/taller than the box.
- Fills are appended after child text, so an edge cannot shift content flow. The existing IPC v2 fill record is reused; the wire version is unchanged. Background fills still paint before text.
- `border:none`, zero effective width and zero-width boxes emit no border. Empty blocks with a border still paint their border box. `MAX_RUNS`, `MAX_HEIGHT`, checked coordinate conversion and the shared block budget remain in force.
- Coverage increases to 20 CSS-subset tests and 96 workspace tests; the worker test also round-trips four border edges after text.

## Independent review

Reviewer: app-owned subagent, provider `deepai`, model `deepai/claude-haiku-5-5`, thinking high, read-only.

- A full-diff review request timed out, and a compact-diff retry failed with `context_length_exceeded`; neither produced a verdict and neither is counted.
- A minimal snippet review returned **APPROVE**. It was given only the exit-time edge-emission block and stated it found no reachable panic, wrong edge geometry or ordering defect.
- Non-blocking finding: corners are covered by both horizontal and vertical fills, so a semi-transparent border colour would blend twice there. The parsed colour set is opaque (`#rgb`, `#rrggbb`, named colours; no alpha), so this has no effect in the current subset. The overlap is documented as a limit rather than hidden.
- This review did not inspect repository files, IPC/app code or the full diff. It is not a full security or rendering review.

## Evidence

- `evidence/verification/border-final2-tests.log`: 96 passed, 0 failed, 0 ignored (42 app, 2 dat, 20 CSS-subset, 4 tree-inspection fixture, 9 static document, 8 tree inspection, 11 IPC).
- `evidence/verification/border-final2-clippy.log`: Clippy clean with `-D warnings`; the gate checked formatting.
- `evidence/verification/border-final2-verification.json` plus `border-final2-*`: `DEVELOPMENT_GATES_OK`, `PROCESS_SMOKE_OK`, `WINDOW_SMOKE_OK paints=1 drawn_text_runs=105 child_rendered=1`, `APP_SMOKE_OK ... paints=10`.
- `evidence/verification/border-final2-background-inspect.log`: fixture has 25 nodes, 39 text runs, 0 unsupported elements/CSS, one limited-layout feature diagnostic.
- `evidence/verification/border-final2-background-smoke.log`: `WINDOW_SMOKE_OK paints=1 drawn_text_runs=34 child_rendered=1`; this executes the GDI background and border fill paths, not a pixel comparison.
- Pinned tree acceptance (`evidence/verification/tree3-border-final.log`): `S04A_ACCEPTANCE_OK`, 3515 pass / 4 unsupported / 0 fail / 0 skip / 0 timeout / 0 harness error; parser behaviour is independent of display-list decoration.

## Limits and non-claims

- Solid axis-aligned block borders only. There is no border radius, dashed/dotted/double/groove style, per-side colour/width, inline border, collapsed border model or background clipping to the border box.
- Four fill records replace a purpose-built border primitive; corner overlaps are intentional, and short or very wide borders are clamped rather than rendered with CSS overflow semantics.
- GDI accepts the fills, but text metrics remain estimates and there is no shaping, selection, pixel equality or website compatibility claim.
- No new `unsafe` was introduced. The review above does not replace the outstanding sandbox, signing and hostile-content work.
