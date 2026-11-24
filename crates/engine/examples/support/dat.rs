//! .dat section boundaries follow the pinned WPT parseDat implementation. Like
//! `BufRead::lines` in the upstream html5ever adapter, CRLF is a line ending;
//! an isolated carriage return remains data. This also handles Git's Windows
//! CRLF working-tree conversion without changing the canonical LF tree output.
//! https://github.com/web-platform-tests/wpt/blob/31d1feb70e2d02f25ae26254caba19c8de0b56e0/html/syntax/parsing/resources/test.js
//! https://github.com/servo/html5ever/blob/492f52a02fd07c2321e59ccff90038e26b9c90bc/rcdom/tests/html-tree-builder.rs

use std::collections::BTreeMap;

#[derive(Debug)]
pub struct Case {
    pub fields: BTreeMap<String, String>,
}

pub fn parse(source: &str) -> Result<Vec<Case>, String> {
    let final_line_ended = source.ends_with('\n');
    let source = source.strip_suffix('\n').unwrap_or(source);
    let lines: Vec<_> = source.split('\n').collect();
    let mut cases = Vec::new();
    let mut fields = BTreeMap::<String, String>::new();
    let mut active: Option<String> = None;
    for (index, raw_line) in lines.iter().enumerate() {
        let line = if index + 1 < lines.len() || final_line_ended {
            raw_line.strip_suffix('\r').unwrap_or(*raw_line)
        } else {
            *raw_line
        };
        if let Some(heading) = line.strip_prefix('#') {
            let heading = heading.trim();
            if !matches!(
                heading,
                "data"
                    | "document"
                    | "errors"
                    | "new-errors"
                    | "document-fragment"
                    | "script-on"
                    | "script-off"
            ) {
                return Err(format!("unknown section at line {}: {heading}", index + 1));
            }
            if heading == "data" && !fields.is_empty() {
                // The blank line separating cases is clipped once, independently of
                // each field's one trailing newline. Never trim meaningful data spaces.
                if let Some(key) = &active {
                    let value = fields.get_mut(key).ok_or("missing active section")?;
                    if !value.is_empty() {
                        value.pop();
                    }
                }
                cases.push(finish(std::mem::take(&mut fields))?);
            } else if fields.is_empty() && heading != "data" {
                return Err(format!("case must start with #data at line {}", index + 1));
            }
            if fields.insert(heading.to_owned(), String::new()).is_some() {
                return Err(format!(
                    "duplicate section at line {}: {heading}",
                    index + 1
                ));
            }
            active = Some(heading.to_owned());
        } else if let Some(key) = &active {
            let value = fields.get_mut(key).ok_or("missing active section")?;
            value.push_str(line);
            if index + 1 < lines.len() {
                value.push('\n');
            }
        } else if !line.is_empty() {
            return Err(format!("content before #data at line {}", index + 1));
        }
    }
    if !fields.is_empty() {
        cases.push(finish(fields)?);
    }
    if cases.is_empty() {
        return Err("suite file contains no cases".into());
    }
    Ok(cases)
}

fn finish(mut fields: BTreeMap<String, String>) -> Result<Case, String> {
    for value in fields.values_mut() {
        if value.ends_with('\n') {
            value.pop();
        }
    }
    if !fields.contains_key("data") || !fields.contains_key("document") {
        return Err("case missing #data or #document".into());
    }
    if fields.contains_key("script-on") && fields.contains_key("script-off") {
        return Err("contradictory scripting flags".into());
    }
    Ok(Case { fields })
}
