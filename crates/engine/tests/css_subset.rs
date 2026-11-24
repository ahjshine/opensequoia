//! Deliberately small CSS subset tests: selectors, cascade, diagnostics and layout effects.
//! This is a documented subset, never a CSS-conformance claim.

use opensequoia_engine::{Color, Diagnostic, DisplayItem, Document, TextRun};

fn text_runs(document: &Document, width: u32) -> Vec<TextRun> {
    document
        .layout(width)
        .unwrap()
        .items
        .into_iter()
        .map(|item| match item {
            DisplayItem::Text(run) => run,
        })
        .collect()
}

fn run_colors(document: &Document) -> Vec<Color> {
    text_runs(document, 800)
        .iter()
        .map(|run| run.color)
        .collect()
}

const RED: Color = Color { r: 255, g: 0, b: 0 };
const BLUE: Color = Color { r: 0, g: 0, b: 255 };
const GREEN: Color = Color { r: 0, g: 128, b: 0 };
const YELLOW: Color = Color {
    r: 255,
    g: 255,
    b: 0,
};

// Value: protects=type/class/id/compound selector specificity; fails_when=the lowest or last
// matching rule wins instead of the most specific one; why_new=no CSS was applied before;
// seam=none; no_claim=CSS cascade conformance beyond this subset.
#[test]
fn type_class_id_and_compound_selectors_cascade_by_specificity() {
    let document = Document::parse(
        "<style>p{color:green}.note{color:blue}p.note{color:red}#lead{color:yellow}</style>\
         <p id=\"lead\" class=\"note\">one</p><p class=\"note\">two</p><p>three</p>",
    )
    .unwrap();
    assert_eq!(run_colors(&document), vec![YELLOW, RED, GREEN]);
}

// Value: protects=inline style precedence and later-rule tie breaking; fails_when=a rule with
// equal specificity wins by document order or inline text is overridden by a sheet rule;
// why_new=only one of the two precedence rules can be observed in a single document;
// seam=none; no_claim=!important or origin precedence (unsupported).
#[test]
fn inline_style_wins_and_equal_specificity_uses_source_order() {
    let document = Document::parse(
        "<style>p{color:red}p{color:blue}</style><p>tie</p><p style=\"color:#0f0\">inline</p>",
    )
    .unwrap();
    assert_eq!(
        run_colors(&document),
        vec![BLUE, Color { r: 0, g: 255, b: 0 }]
    );
}

// Value: protects=comma-separated selector lists; fails_when=only the first selector of a list
// applies; why_new=single-selector rules would still pass; seam=none; no_claim=selector groups
// with combinators or pseudo-classes.
#[test]
fn comma_separated_selector_lists_apply_to_each_selector() {
    let document =
        Document::parse("<style>h1, .note{color:#ff0000}</style><h1>A</h1><p class=\"note\">B</p>")
            .unwrap();
    assert_eq!(run_colors(&document), vec![RED, RED]);
}

// Value: protects=bounded, non-fatal skipping with explicit unsupported accounting;
// fails_when=a bad declaration aborts the document, silently applies, or is uncounted;
// why_new=objectively wrong CSS must stay visible as a gap; seam=none;
// no_claim=full CSS error recovery.
#[test]
fn unsupported_css_is_skipped_without_failing_and_is_counted() {
    let document = Document::parse(
        "<style>p{color:#12}p{font-size:1.5em}div > p{color:red}a:hover{color:red}\\
         p{width:10px}p{margin:1px 2px 3px 4px 5px}</style>\
         <p style=\"border:1px solid red;font-size:20px\">ok</p>",
    )
    .unwrap();
    assert!(document.diagnostics().contains(&Diagnostic::UnsupportedCss));
    assert_eq!(document.unsupported_css_declarations(), 7);
    let runs = text_runs(&document, 800);
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].font_size, 20);
    assert_eq!(runs[0].text, "ok");
}

// Value: protects=the supported property set without false unsupported reports; fails_when=a
// supported declaration or a fully supported sheet is still counted as unsupported;
// why_new=counters could hide regressions behind one deduplicated diagnostic; seam=none;
// no_claim=CSS conformance for properties outside this subset.
#[test]
fn supported_subset_reports_no_unsupported_css() {
    let document = Document::parse(
        "<style>p.note{color:#336699;background-color:#ffffff;font-size:18px;\
         font-weight:bold;text-align:right;margin:4px 8px;padding:2px 3px 4px 5px;\
         border:2px;border-style:solid;border-color:#000000;display:block}</style>\
         <p class=\"note\">x</p><p style=\"display:inline\">y</p>",
    )
    .unwrap();
    assert_eq!(document.unsupported_css_declarations(), 0);
    assert!(!document.diagnostics().contains(&Diagnostic::UnsupportedCss));
}

// Value: protects=styled text metrics reaching the display list; fails_when=size, weight,
// colour or alignment is parsed but not laid out; why_new=parse-only tests cannot show the
// render path; seam=none; no_claim=shaping or real font metrics.
#[test]
fn layout_applies_font_size_bold_color_alignment_and_spacing() {
    let document = Document::parse(
        "<style>p{font-size:20px;font-weight:bold;color:#336699;text-align:center;\
         margin:8px 0 0 24px;padding:4px 0 0 6px}</style><p>Hi</p>",
    )
    .unwrap();
    let runs = text_runs(&document, 800);
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].font_size, 20);
    assert!(runs[0].bold);
    assert_eq!(
        runs[0].color,
        Color {
            r: 0x33,
            g: 0x66,
            b: 0x99
        }
    );
    // 16px page margin + 24px left margin + 6px left padding, then centred in 738px.
    assert_eq!((runs[0].x, runs[0].y), (403, 28));
}

// Value: protects=1/2/3/4-value margin and padding expansion; fails_when=all sides take the
// first value; why_new=only single-value shorthands could be tested with one side;
// seam=none; no_claim=percentage, em or auto lengths.
#[test]
fn margin_and_padding_shorthands_offset_the_content_box() {
    let document =
        Document::parse("<style>p{margin:8px 0 0 24px;padding:4px 3px 2px 6px}</style><p>Hi</p>")
            .unwrap();
    let runs = text_runs(&document, 800);
    assert_eq!((runs[0].x, runs[0].y), (46, 28));
}

// Value: protects=1-value border shorthand and four-value margin expansion feeding layout;
// fails_when=border width is ignored in the content inset; why_new=only padding was covered;
// seam=no painted border (v1 IPC has no decoration record); no_claim=border painting.
#[test]
fn border_width_insets_content_inside_four_value_margins() {
    let document =
        Document::parse("<style>p{border:3px;margin:1px 2px 3px 4px}</style><p>Hi</p>").unwrap();
    let runs = text_runs(&document, 800);
    assert_eq!((runs[0].x, runs[0].y), (23, 20));
}

// Value: protects=display none subtree suppression and display block override; fails_when=hidden
// text still reaches the display list; why_new=display was not implemented at all;
// seam=none; no_claim=inline formatting contexts or float.
#[test]
fn display_none_suppresses_the_subtree_and_block_forces_a_line() {
    let document = Document::parse(
        "<style>.gone{display:none}p{display:inline}</style>\
         <p class=\"gone\">hidden</p><p>visible</p>",
    )
    .unwrap();
    let runs = text_runs(&document, 800);
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].text, "visible");
}

// Value: protects=inheritance of colour/size and override of the <b> default weight;
// fails_when=CSS text properties stop at the element they are declared on; why_new=every
// earlier case declares properties on the text-bearing element itself; seam=none;
// no_claim=inheritance of box properties (not inherited).
#[test]
fn inherited_text_properties_reach_descendants_and_can_revert_bold() {
    let document = Document::parse(
        "<style>body{font-size:24px;color:#ff0000}b{font-weight:normal}</style>\
         <div><b>x</b></div>",
    )
    .unwrap();
    let runs = text_runs(&document, 800);
    assert_eq!(runs[0].font_size, 24);
    assert!(!runs[0].bold);
    assert_eq!(runs[0].color, RED);
}

// Value: protects=the unstyled document contract (metrics, colour, no CSS diagnostics);
// fails_when=CSS defaults change existing pages; why_new=styling must not silently regress
// documents without CSS; seam=none; no_claim=byte-identical rendering of every input.
#[test]
fn documents_without_css_keep_default_text_metrics_and_diagnostics() {
    let document = Document::parse("<h1>Heading</h1><p>Body</p>").unwrap();
    let runs = text_runs(&document, 800);
    assert_eq!(runs[0].text, "Heading");
    assert_eq!(runs[0].font_size, 32);
    assert!(runs[0].bold);
    assert_eq!(
        runs[0].color,
        Color {
            r: 28,
            g: 41,
            b: 35
        }
    );
    assert!(!document.diagnostics().contains(&Diagnostic::UnsupportedCss));
    assert_eq!(document.unsupported_css_declarations(), 0);
}

// Value: protects=the IPC v1 display bound (font_size 1..=256) against CSS values the wire
// cannot carry; fails_when=a 300px declaration emits a run that opensequoia-ipc rejects, or the
// supported 256px boundary is itself rejected; why_new=CSS previously allowed 512px while IPC
// v1 allows 256; seam=engine to IPC display contract; no_claim=large-type fidelity beyond the
// protocol bound.
#[test]
fn font_size_stays_within_the_display_protocol_bound() {
    let oversized = Document::parse("<p style=\"font-size:300px\">x</p>").unwrap();
    assert!(
        oversized
            .diagnostics()
            .contains(&Diagnostic::UnsupportedCss)
    );
    assert_eq!(oversized.unsupported_css_declarations(), 1);
    assert_eq!(text_runs(&oversized, 800)[0].font_size, 16);

    let boundary = Document::parse("<p style=\"font-size:256px\">x</p>").unwrap();
    assert_eq!(boundary.unsupported_css_declarations(), 0);
    assert_eq!(text_runs(&boundary, 800)[0].font_size, 256);
}

// Value: protects=a single shared rule budget across every <style> element; fails_when=each
// stylesheet gets its own 4096 rules and later element matching scans an unbounded rule set;
// why_new=the cap was applied per stylesheet; seam=hostile local input to style computation;
// no_claim=linear-time CSS matching.
#[test]
fn stylesheet_rule_budget_is_shared_across_all_style_elements() {
    let rules = "p{}".repeat(4096);
    let html = format!("<style>{rules}</style><style>{rules}</style><p>x</p>");
    let document = Document::parse(&html).unwrap();
    assert_eq!(document.unsupported_css_declarations(), 1);
    assert_eq!(text_runs(&document, 800)[0].text, "x");
}

// Value: protects=ASCII case-insensitive values plus CSS comment removal in inline styles;
// fails_when="COLOR: RED" or an inline comment makes a supported declaration unsupported;
// why_new=values were matched case-sensitively and comments were only stripped in sheets;
// seam=none; no_claim=CSS escape or Unicode case handling.
#[test]
fn values_are_case_insensitive_and_inline_comments_are_ignored() {
    let document = Document::parse(
        "<p style=\"COLOR: RED; FONT-WEIGHT: BOLD; /* note */ TEXT-ALIGN: CENTER\">Hi</p>",
    )
    .unwrap();
    assert_eq!(document.unsupported_css_declarations(), 0);
    let runs = text_runs(&document, 800);
    assert_eq!(runs[0].color, RED);
    assert!(runs[0].bold);
}

// Value: protects=border:none resetting a previously declared width; fails_when=the earlier
// border width still insets content or the valid declaration is counted unsupported;
// why_new=only solid borders and a bare width were handled; seam=no border painting;
// no_claim=the full CSS border shorthand.
#[test]
fn border_none_resets_width_without_becoming_unsupported() {
    let document = Document::parse("<p style=\"border:9px;border:none\">x</p>").unwrap();
    assert_eq!(document.unsupported_css_declarations(), 0);
    assert_eq!(text_runs(&document, 800)[0].x, 16);
}

// Value: protects=html/body block-box defaults so body margins reach descendants;
// fails_when=body stays inline and its margin is discarded; why_new=the UA defaults
// omitted the document/body boxes; seam=no default 8px body margin; no_claim=full UA CSS.
#[test]
fn html_and_body_are_block_boxes() {
    let document = Document::parse("<body style=\"margin:20px\"><p>x</p></body>").unwrap();
    assert_eq!(text_runs(&document, 800)[0].x, 36);
}
