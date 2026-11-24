use opensequoia_engine::{
    DisplayItem, Document, Error, FragmentContext, MAX_TREE_DUMP_BYTES, Namespace, ParseOptions,
};

// Value: protects=canonical non-rendered nodes, entities and UTF-16 attribute order; fails_when=inspection drops or reorders required content; why_new=layout tests omit these nodes; seam=none; no_claim=full WPT coverage.
#[test]
fn canonical_tree_preserves_doctype_comments_entities_and_attribute_order() {
    let document = Document::parse("<!doctype html PUBLIC 'p' 's'><!--before--><p z='&amp;' \u{e000}='bmp' a='1' \u{10000}='astral'>नमस्ते&amp;<!--inside-->").unwrap();
    assert_eq!(
        document.tree_dump().unwrap(),
        concat!(
            "| <!DOCTYPE html \"p\" \"s\">\n",
            "| <!-- before -->\n| <html>\n|   <head>\n|   <body>\n",
            "|     <p>\n|       a=\"1\"\n|       z=\"&\"\n",
            "|       \u{10000}=\"astral\"\n|       \u{e000}=\"bmp\"\n",
            "|       \"नमस्ते&\"\n|       <!-- inside -->"
        )
    );
}

// Value: protects=exact nested-template render exclusion; fails_when=any hidden/nested run leaks or the shown run is lost; why_new=previous len<3 allowed one hidden run; seam=none; no_claim=live template DOM APIs.
#[test]
fn template_contents_are_inspected_but_are_not_rendered() {
    let document =
        Document::parse("<template><p>hidden<template><b>nested</b></template></template><p>shown")
            .unwrap();
    assert_eq!(
        document.tree_dump().unwrap(),
        concat!(
            "| <html>\n|   <head>\n|     <template>\n|       content\n",
            "|         <p>\n|           \"hidden\"\n|           <template>\n",
            "|             content\n|               <b>\n|                 \"nested\"\n",
            "|   <body>\n|     <p>\n|       \"shown\""
        )
    );
    let items = document.layout(800).unwrap().items;
    assert_eq!(items.len(), 1);
    let [DisplayItem::Text(run)] = items.as_slice() else {
        panic!("expected exactly one text run");
    };
    assert_eq!(run.text, "shown");
}

// Value: protects=SVG/MathML and namespaced attribute spelling; fails_when=namespace prefixes are dropped; why_new=HTML-only fixtures miss foreign content; seam=none; no_claim=SVG or MathML rendering.
#[test]
fn foreign_namespace_names_and_attributes_are_preserved() {
    let document =
        Document::parse("<svg><g xml:lang='en' xlink:href='#x'></g></svg><math><mi>x</mi></math>")
            .unwrap();
    let output = document.tree_dump().unwrap();
    assert!(output.contains("<svg g>\n|         xlink href=\"#x\"\n|         xml lang=\"en\""));
    assert!(output.contains("<math mi>"));
}

// Value: protects=bounded stack-safe inspection of deep trees; fails_when=indentation grows without the output cap or recursion returns; why_new=small trees cannot trigger quadratic output; seam=none; no_claim=memory or time benchmarking.
#[test]
fn deeply_indented_inspection_rejects_before_unbounded_output_growth() {
    let input = format!("{}leaf{}", "<div>".repeat(8_000), "</div>".repeat(8_000));
    let document = Document::parse(&input).unwrap();
    assert_eq!(document.tree_dump(), Err(Error::TreeDumpLimit));
    let shallow = Document::parse("<div>leaf</div>")
        .unwrap()
        .tree_dump()
        .unwrap();
    assert!(shallow.len() < MAX_TREE_DUMP_BYTES);
}

// Value: protects=explicit scripting parse modes without script execution; fails_when=noscript parsing ignores the flag or script text executes; why_new=default parsing covers only scripting-off; seam=none; no_claim=JavaScript support.
#[test]
fn parser_scripting_flag_changes_noscript_tokens_without_executing_scripts() {
    let input =
        "<body><noscript><b>inside</b></noscript><script>document.write('executed')</script>after";
    let default = Document::parse(input).unwrap().tree_dump().unwrap();
    let configured = Document::parse_with_options(
        input,
        &ParseOptions {
            scripting_enabled: true,
            fragment: None,
        },
    )
    .unwrap()
    .tree_dump()
    .unwrap();
    assert!(default.contains("<noscript>\n|       <b>"));
    assert!(configured.contains("<noscript>\n|       \"<b>inside</b>\""));
    assert!(!configured.contains("\"executed\""));
    assert_eq!(
        Document::parse_with_options(input, &ParseOptions::default())
            .unwrap()
            .tree_dump()
            .unwrap(),
        default
    );
}

// Value: protects=real HTML/SVG fragment parsing and invalid-context rejection; fails_when=document parsing is substituted or context markup is accepted; why_new=document tests miss fragment-only modes; seam=none; no_claim=contextual ancestors or host DOM APIs.
#[test]
fn fragment_context_controls_html_and_foreign_tree_construction() {
    let html = ParseOptions {
        scripting_enabled: false,
        fragment: Some(FragmentContext {
            namespace: Namespace::Html,
            local_name: "tbody".into(),
        }),
    };
    assert_eq!(
        Document::parse_with_options("<tr><td>cell", &html)
            .unwrap()
            .tree_dump()
            .unwrap(),
        "| <tr>\n|   <td>\n|     \"cell\""
    );
    let svg = ParseOptions {
        scripting_enabled: false,
        fragment: Some(FragmentContext {
            namespace: Namespace::Svg,
            local_name: "path".into(),
        }),
    };
    assert_eq!(
        Document::parse_with_options("<g/>X", &svg)
            .unwrap()
            .tree_dump()
            .unwrap(),
        "| <svg g>\n| \"X\""
    );
    let invalid = ParseOptions {
        scripting_enabled: false,
        fragment: Some(FragmentContext {
            namespace: Namespace::Html,
            local_name: "<body>".into(),
        }),
    };
    assert!(matches!(
        Document::parse_with_options("x", &invalid),
        Err(Error::InvalidContext)
    ));
}

fn assert_selectedcontent_tree(input: &str, expected: &str) {
    for scripting_enabled in [false, true] {
        let document = Document::parse_with_options(
            input,
            &ParseOptions {
                scripting_enabled,
                fragment: None,
            },
        )
        .unwrap();
        assert_eq!(
            document.tree_dump().unwrap(),
            expected,
            "scripting_enabled={scripting_enabled}"
        );
    }
}

// Value: protects=pinned webkit02.dat#44 option content cloning at EOF; fails_when=selectedcontent stays empty or the original option is moved; why_new=existing inspection tests omit selectedcontent; seam=none; no_claim=live selection or JavaScript behavior.
#[test]
fn selectedcontent_clones_default_option_text_at_eof() {
    assert_selectedcontent_tree(
        "<select><button><selectedcontent></button><option>X",
        concat!(
            "| <html>\n|   <head>\n|   <body>\n|     <select>\n",
            "|       <button>\n|         <selectedcontent>\n|           \"X\"\n",
            "|       <option>\n|         \"X\""
        ),
    );
}

// Value: protects=pinned webkit02.dat#45 deep cloned recovered formatting tree; fails_when=cloning omits descendants, changes order or aliases the option; why_new=text-only cloning cannot catch formatting reconstruction errors; seam=none; no_claim=full DOM cloning or formatting conformance.
#[test]
fn selectedcontent_clones_recovered_option_formatting_at_eof() {
    assert_selectedcontent_tree(
        "<select><button><selectedcontent></button><option>x<i>i<b>ib</i>b",
        concat!(
            "| <html>\n|   <head>\n|   <body>\n|     <select>\n",
            "|       <button>\n|         <selectedcontent>\n",
            "|           \"x\"\n|           <i>\n|             \"i\"\n",
            "|             <b>\n|               \"ib\"\n|           <b>\n",
            "|             \"b\"\n|       <option>\n|         \"x\"\n",
            "|         <i>\n|           \"i\"\n|           <b>\n",
            "|             \"ib\"\n|         <b>\n|           \"b\""
        ),
    );
}
