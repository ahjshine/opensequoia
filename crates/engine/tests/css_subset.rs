//! Deliberately small CSS subset tests: selectors, cascade, diagnostics and layout effects.
//! This is a documented subset, never a CSS-conformance claim.

use opensequoia_engine::{Color, Diagnostic, DisplayItem, Document, TextRun};

fn text_runs(document: &Document, width: u32) -> Vec<TextRun> {
    document
        .layout(width)
        .unwrap()
        .items
        .into_iter()
        .filter_map(|item| match item {
            DisplayItem::Text(run) => Some(run),
            DisplayItem::Fill(_) => None,
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
         <p style=\"border:1px dashed red;font-size:20px\">ok</p>",
    )
    .unwrap();
    assert!(document.diagnostics().contains(&Diagnostic::UnsupportedCss));
    assert_eq!(document.unsupported_css_declarations(), 7);
    let runs = text_runs(&document, 800);
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].font_size, 20);
    assert_eq!(runs[0].text, "ok");
    let list = document.layout(800).unwrap();
    assert!(
        list.items
            .iter()
            .all(|item| !matches!(item, DisplayItem::Fill(_)))
    );
}

// Value: protects=the supported property set without false unsupported reports; fails_when=a
// supported declaration or a fully supported sheet is still counted as unsupported;
// why_new=counters could hide regressions behind one deduplicated diagnostic; seam=none;
// no_claim=CSS conformance for properties outside this subset.
#[test]
fn supported_subset_reports_no_unsupported_css() {
    let document = Document::parse(
        "<style>p.note{color:#336699;background:#fefefe;background-color:#ffffff;font-size:18px;\
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
// seam=border edges are separate fill records, not a border primitive; no_claim=border style fidelity.
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

// Value: protects=single-colour background shorthand reaching the fill list;
// fails_when=the shorthand remains unsupported or parses without painting;
// why_new=only background-color was accepted; seam=one colour token only, no images,
// gradients, alpha or multi-layer values; no_claim=full CSS background shorthand.
#[test]
fn background_shorthand_applies_one_color() {
    let document =
        Document::parse("<div style=\"background:#102030;margin:0;padding:0\">x</div>").unwrap();
    assert_eq!(document.unsupported_css_declarations(), 0);
    let list = document.layout(400).unwrap();
    let fills: Vec<_> = list
        .items
        .iter()
        .filter_map(|item| match item {
            DisplayItem::Fill(fill) => Some(fill),
            DisplayItem::Text(_) => None,
        })
        .collect();
    assert_eq!(fills.len(), 1);
    assert_eq!(
        fills[0].color,
        Color {
            r: 16,
            g: 32,
            b: 48,
        }
    );
}

// Value: protects=background:none clearing a prior colour and multi-token rejection;
// fails_when=none still paints, or an image/multi-layer value is partially applied;
// why_new=silent partial backgrounds are worse than an explicit unsupported count;
// seam=no images, gradients, alpha or multiple layers; no_claim=full CSS background.
#[test]
fn background_shorthand_none_clears_and_multi_token_rejects() {
    let cleared =
        Document::parse("<p style=\"background-color:#112233;background:none\">x</p>").unwrap();
    assert_eq!(cleared.unsupported_css_declarations(), 0);
    assert!(
        !cleared
            .layout(400)
            .unwrap()
            .items
            .iter()
            .any(|item| matches!(item, DisplayItem::Fill(_)))
    );

    let rejected = Document::parse("<p style=\"background:#ffffff url(x)\">x</p>").unwrap();
    assert_eq!(rejected.unsupported_css_declarations(), 1);
    assert!(
        !rejected
            .layout(400)
            .unwrap()
            .items
            .iter()
            .any(|item| matches!(item, DisplayItem::Fill(_)))
    );
}

// Value: protects=the 1-3 token border shorthand applying width, style and colour together;
// fails_when=the shorthand still reports unsupported or only the width reaches layout;
// why_new=the earlier parser accepted only a bare width; seam=solid/none styles only, no
// radius or dashed/dotted/double; no_claim=full CSS border shorthand.
#[test]
fn border_shorthand_applies_width_style_and_color() {
    let document =
        Document::parse("<div style=\"border:3px solid #112233;padding:0\">x</div>").unwrap();
    assert_eq!(document.unsupported_css_declarations(), 0);
    let list = document.layout(400).unwrap();
    let fills: Vec<_> = list
        .items
        .iter()
        .filter_map(|item| match item {
            DisplayItem::Fill(fill) => Some(fill),
            DisplayItem::Text(_) => None,
        })
        .collect();
    assert_eq!(fills.len(), 4);
    let border = Color {
        r: 17,
        g: 34,
        b: 51,
    };
    assert!(fills.iter().all(|fill| fill.color == border));
    assert_eq!((fills[0].width, fills[0].height), (368, 3));
}

// Value: protects=border:none resetting a previously declared width; fails_when=the earlier
// border width still insets content or the valid declaration is counted unsupported;
// why_new=only solid borders and a bare width were handled; seam=no border-radius or
// multi-token shorthand; no_claim=the full CSS border shorthand.
#[test]
fn border_none_resets_width_without_becoming_unsupported() {
    let document = Document::parse("<p style=\"border:9px;border:none\">x</p>").unwrap();
    assert_eq!(document.unsupported_css_declarations(), 0);
    assert_eq!(text_runs(&document, 800)[0].x, 16);
    let list = document.layout(800).unwrap();
    assert!(
        !list
            .items
            .iter()
            .any(|item| matches!(item, DisplayItem::Fill(_)))
    );
}

// Value: protects=html/body block-box defaults so body margins reach descendants;
// fails_when=body stays inline and its margin is discarded; why_new=the UA defaults
// omitted the document/body boxes; seam=no default 8px body margin; no_claim=full UA CSS.
#[test]
fn html_and_body_are_block_boxes() {
    let document = Document::parse("<body style=\"margin:20px\"><p>x</p></body>").unwrap();
    assert_eq!(text_runs(&document, 800)[0].x, 36);
}

// Value: protects=block background geometry and paint order behind child text;
// fails_when=background is absent, covers only the content box, or paints over text;
// why_new=background-color is parsed but needs an actual display primitive;
// seam=block boxes only, no inline/background propagation; no_claim=CSS painting.
#[test]
fn block_background_emits_a_fill_behind_its_content() {
    let document = Document::parse(
        "<body style=\"background-color:#102030\"><p style=\"margin:10px\">x</p></body>",
    )
    .unwrap();
    let list = document.layout(400).unwrap();
    let DisplayItem::Fill(ref fill) = list.items[0] else {
        panic!("background fill must be the first item");
    };
    assert_eq!(
        (fill.x, fill.y, fill.width, fill.color),
        (
            16,
            16,
            368,
            Color {
                r: 0x10,
                g: 0x20,
                b: 0x30
            }
        )
    );
    assert!(fill.height > 0);
    assert!(fill.height as i32 + fill.y <= list.height as i32);
    assert!(
        list.items
            .iter()
            .skip(1)
            .any(|item| matches!(item, DisplayItem::Text(run) if run.text == "x"))
    );
}

// Value: protects=block border edges painted after text without changing content flow;
// fails_when=the border is absent, painted under text, or uses the wrong box; why_new=the
// subset parsed border width/colour but had no border display record; seam=block borders
// only, four axis-aligned fills, no border-radius/style fidelity; no_claim=CSS border
// conformance or exact pixel geometry.
#[test]
fn block_border_paints_edges_after_text() {
    let document = Document::parse(
        "<body style=\"background-color:#ffffff;border-width:4px;border-style:solid;border-color:#112233;padding:6px;margin:8px\"><p>x</p></body>",
    )
    .unwrap();
    let list = document.layout(400).unwrap();
    let fills: Vec<(usize, &opensequoia_engine::FillRect)> = list
        .items
        .iter()
        .enumerate()
        .filter_map(|(index, item)| match item {
            DisplayItem::Fill(fill) => Some((index, fill)),
            DisplayItem::Text(_) => None,
        })
        .collect();
    let text_index = list
        .items
        .iter()
        .position(|item| matches!(item, DisplayItem::Text(_)))
        .expect("text missing");
    assert_eq!(fills.len(), 5, "one background and four border edges");
    let (background_index, background) = fills[0];
    assert!(background_index < text_index);
    assert_eq!(
        background.color,
        Color {
            r: 255,
            g: 255,
            b: 255
        }
    );
    assert_eq!(
        (background.x, background.y, background.width),
        (24, 24, 352)
    );
    let border = Color {
        r: 17,
        g: 34,
        b: 51,
    };
    let edges = &fills[1..];
    assert!(
        edges
            .iter()
            .all(|(index, fill)| *index > text_index && fill.color == border)
    );
    let (_, top) = edges[0];
    let (_, bottom) = edges[1];
    let (_, left) = edges[2];
    let (_, right) = edges[3];
    assert_eq!((top.x, top.y, top.width, top.height), (24, 24, 352, 4));
    assert_eq!(
        (bottom.x, bottom.y, bottom.width, bottom.height),
        (24, background.y + background.height as i32 - 4, 352, 4)
    );
    assert_eq!(
        (left.x, left.y, left.width, left.height),
        (24, 24, 4, background.height)
    );
    assert_eq!(
        (right.x, right.y, right.width, right.height),
        (372, 24, 4, background.height)
    );
}

// Value: protects=an empty block still paints its solid border box; fails_when=the border
// disappears with the empty background or uses zero height; why_new=border and background
// share a box but only the background had an empty-block regression; seam=block borders
// only, integer fills; no_claim=border style fidelity.
#[test]
fn empty_block_still_paints_its_border() {
    let document = Document::parse(
        "<div style=\"border-width:3px;border-style:solid;border-color:#112233;padding:2px\"></div>",
    )
    .unwrap();
    let list = document.layout(400).unwrap();
    let fills: Vec<_> = list
        .items
        .iter()
        .filter_map(|item| match item {
            DisplayItem::Fill(fill) => Some(fill),
            DisplayItem::Text(_) => None,
        })
        .collect();
    assert_eq!(fills.len(), 4);
    let border = Color {
        r: 17,
        g: 34,
        b: 51,
    };
    assert!(fills.iter().all(|fill| fill.color == border));
    assert_eq!((fills[0].width, fills[0].height), (368, 3));
    assert_eq!((fills[1].width, fills[1].height), (368, 3));
    assert_eq!((fills[2].width, fills[2].height), (3, 10));
    assert_eq!((fills[3].width, fills[3].height), (3, 10));
}

// Value: protects=empty blocks do not emit zero-height wire records;
// fails_when=a zero-height fill survives into the display list; why_new=the fill
// height is only known at exit; seam=zero-size boxes; no_claim=full CSS layout.
#[test]
fn empty_background_blocks_emit_no_fill() {
    let document = Document::parse("<body style=\"background-color:#fff\"></body>").unwrap();
    assert!(
        document
            .layout(400)
            .unwrap()
            .items
            .iter()
            .all(|item| matches!(item, DisplayItem::Text(_)))
    );
    // An empty fill after text and before another block must not corrupt the
    // current line boundary when it is removed.
    let document =
        Document::parse("<body><p>a</p><div style=\"background-color:#fff\"></div><p>b</p></body>")
            .unwrap();
    let runs = text_runs(&document, 400);
    assert_eq!(
        runs.iter().map(|run| run.text.as_str()).collect::<Vec<_>>(),
        ["a", "b"]
    );
}
