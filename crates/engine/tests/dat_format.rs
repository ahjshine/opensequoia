#[path = "../examples/support/dat.rs"]
mod dat;

// Value: protects=upstream CRLF handling without data loss; fails_when=CR is retained in expected trees or isolated CR is deleted; why_new=Windows checkout exposed mass false mismatches; seam=none; no_claim=full corpus compatibility.
#[test]
fn dat_sections_normalize_crlf_but_preserve_other_data_whitespace() {
    let source = "#data\r\n\r\nx\r\ny\rz\n\n#errors\r\n#document\r\n| \"\nx\r\ny\rz\n\"\n\n#data\nx\n#errors\n#document\n| \"x\"\n";
    let cases = dat::parse(source).unwrap();
    assert_eq!(cases.len(), 2);
    assert_eq!(cases[0].fields["data"], "\nx\ny\rz\n");
    assert_eq!(cases[0].fields["document"], "| \"\nx\ny\rz\n\"");
    assert_eq!(cases[1].fields["data"], "x");
    assert_eq!(cases[1].fields["document"], "| \"x\"");
}

// Value: protects=empty data, fragment/script metadata and both error sections; fails_when=original #errors is dropped or other valid flags are lost; why_new=prior fixture asserted only #new-errors; seam=none; no_claim=parse-error text conformance.
#[test]
fn dat_empty_data_fragment_flags_and_error_sections_are_kept() {
    let cases = dat::parse("#data\n\n#errors\nold\n#new-errors\nnew\n#document-fragment\nsvg path\n#script-on\n#document\n").unwrap();
    assert_eq!(cases[0].fields["data"], "");
    assert_eq!(cases[0].fields["document-fragment"], "svg path");
    assert_eq!(cases[0].fields["errors"], "old");
    assert_eq!(cases[0].fields["new-errors"], "new");
    assert!(cases[0].fields.contains_key("script-on"));
}

// Value: protects=final isolated CR as DAT content; fails_when=CR is stripped without a following LF; why_new=CRLF coverage did not exercise an unterminated final line; seam=none; no_claim=browser DOM behavior.
#[test]
fn dat_final_isolated_carriage_return_is_not_a_line_ending() {
    let cases = dat::parse("#data\nx\n#document\n| \"x\"\r").unwrap();
    assert_eq!(cases[0].fields["document"], "| \"x\"\r");
}

// Value: protects=fail-closed DAT census; fails_when=malformed sections silently drop cases or flags; why_new=acceptance requires complete case identities; seam=none; no_claim=valid corpus tree correctness.
#[test]
fn malformed_dat_files_fail_instead_of_losing_cases_or_sections() {
    for source in [
        "",
        "#errors\nx",
        "#data\nx",
        "#data\nx\n#document\nx\n#document\ny",
        "#data\nx\n#script-on\n#script-off\n#document\nx",
        "#data\nx\n#unexpected\n#document\nx",
    ] {
        assert!(dat::parse(source).is_err(), "{source}");
    }
}
