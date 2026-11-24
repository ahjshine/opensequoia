use opensequoia_engine::{Diagnostic, DisplayItem, Document, Error, MAX_INPUT_BYTES, MAX_NODES};

fn text(document: &Document, width: u32) -> String {
    document
        .layout(width)
        .unwrap()
        .items
        .into_iter()
        .map(|item| match item {
            DisplayItem::Text(run) => run.text,
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[test]
fn html_tree_builder_repairs_paragraphs_and_preserves_entities_unicode() {
    let document = Document::parse("<!doctype html><p>Hello &amp; नमस्ते<p>next").unwrap();
    assert_eq!(text(&document, 800), "Hello & नमस्ते next");
    assert!(document.node_count() >= 7);
}

#[test]
fn non_rendered_content_cannot_leak_into_display_list() {
    let document = Document::parse("<head><title>hidden</title><style>style hidden</style></head><body><script>script hidden</script><template>template hidden</template><p>visible</p>").unwrap();
    assert_eq!(text(&document, 800), "visible");
    assert!(document.diagnostics().contains(&Diagnostic::UnsupportedCss));
}

#[test]
fn input_and_node_limits_reject_instead_of_returning_truncated_documents() {
    assert!(matches!(
        Document::parse(&"a".repeat(MAX_INPUT_BYTES + 1)),
        Err(Error::InputTooLarge)
    ));
    let input = "<i></i>".repeat(MAX_NODES);
    assert!(input.len() < MAX_INPUT_BYTES);
    assert!(matches!(Document::parse(&input), Err(Error::NodeLimit)));
}

#[test]
fn deep_tree_traversal_and_drop_do_not_recurse() {
    let depth = 8_000;
    let input = format!(
        "{}<p>leaf</p>{}",
        "<div>".repeat(depth),
        "</div>".repeat(depth)
    );
    let document = Document::parse(&input).unwrap();
    assert_eq!(text(&document, 800), "leaf");
    drop(document);
}

#[test]
fn viewport_rejection_and_wrapping_have_deterministic_bounds() {
    let document =
        Document::parse("<h1>Heading</h1><p>abcdefghijklmnopqrstuvwxyz words</p>").unwrap();
    assert_eq!(document.layout(0), Err(Error::InvalidViewport));
    assert_eq!(document.layout(8193), Err(Error::InvalidViewport));
    let list = document.layout(96).unwrap();
    assert_eq!(list, document.layout(96).unwrap());
    assert!(list.items.len() > 2);
    for item in list.items {
        let DisplayItem::Text(run) = item;
        assert!(run.x >= 0 && run.y >= 0);
        assert!(run.x as u32 + run.width <= list.width);
        assert!(run.y as u32 + run.height <= list.height);
    }
}

#[test]
fn foster_parenting_and_misnested_formatting_terminate_with_text() {
    let document =
        Document::parse("<table>before<tr><td>cell</table><b>bold<i>both</b>after</i>").unwrap();
    let output = text(&document, 800);
    assert!(output.contains("before"));
    assert!(output.contains("cell"));
    assert!(output.contains("bold"));
    assert!(output.contains("after"));
}

#[test]
fn unsupported_css_and_active_elements_are_explicit() {
    // `width` stays outside the documented CSS subset, so the diagnostic must remain visible
    // while the supported `color` declaration still applies to the same element.
    let document = Document::parse(
        "<p style='color:red;width:10px'>shown</p><input value='x'><a href='https://example.com'>link</a>",
    )
    .unwrap();
    assert!(document.diagnostics().contains(&Diagnostic::UnsupportedCss));
    assert!(
        document
            .diagnostics()
            .contains(&Diagnostic::UnsupportedElement("input".into()))
    );
    assert!(
        document
            .diagnostics()
            .contains(&Diagnostic::UnsupportedElement("a".into()))
    );
    assert!(text(&document, 800).contains("shown"));
}

#[test]
fn unicode_chunk_boundaries_and_html_space_rules_are_preserved() {
    let input = format!("<p>{} नमस्ते&nbsp;世界</p>", "x".repeat(248));
    let document = Document::parse(&input).unwrap();
    let output = text(&document, 8192);
    assert!(output.contains("नमस्ते\u{00a0}世界"));
    let document = Document::parse("<p>a<span>b</span> c\n\t d</p>").unwrap();
    let list = document.layout(800).unwrap();
    let runs: Vec<_> = list
        .items
        .iter()
        .map(|item| match item {
            DisplayItem::Text(run) => run,
        })
        .collect();
    assert_eq!(runs[0].x + runs[0].width as i32, runs[1].x);
    assert!(runs[2].x > runs[1].x + runs[1].width as i32);
}

#[test]
fn generated_malformed_token_sequences_keep_display_bounds() {
    let tokens = [
        "<p>",
        "</div>",
        "<table>",
        "<b>",
        "</i>",
        "<template>",
        "</template>",
        "<script>",
        "</script>",
        "&amp;",
        "नमस्ते",
        "\0",
        " ",
        "<br>",
        "text",
    ];
    for seed in 1_u32..=256 {
        let mut state = seed;
        let mut input = String::new();
        for _ in 0..24 {
            state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            input.push_str(tokens[state as usize % tokens.len()]);
        }
        let document = Document::parse(&input).unwrap();
        let list = document.layout(64 + seed % 320).unwrap();
        for item in list.items {
            let DisplayItem::Text(run) = item;
            assert!(run.x >= 0 && run.y >= 0);
            assert!(run.x as u32 + run.width <= list.width);
            assert!(run.y as u32 + run.height <= list.height);
        }
    }
}
