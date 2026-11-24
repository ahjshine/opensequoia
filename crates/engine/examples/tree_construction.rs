//! Isolated, tree-only HTML conformance adapter; this is development tooling.
//! Usage: tree_construction --suite DIR --suite-revision COMMIT --report FILE
//! [--scratch-dir target/tree3] [--timeout-ms 5000].
//! Source SHA-256 inventory is recorded alongside the report by the acceptance oracle.
#![forbid(unsafe_code)]

#[path = "support/dat.rs"]
mod dat;

use std::ffi::OsString;
use std::fs::{self, File};
use std::io::{BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use opensequoia_engine::{Document, FragmentContext, MAX_TREE_DUMP_BYTES, Namespace, ParseOptions};

const MAX_DAT_BYTES: u64 = 1024 * 1024;

fn read_text(path: &Path, limit: u64) -> Result<String, String> {
    let file = File::open(path).map_err(|error| format!("open failed: {error}"))?;
    let mut bytes = Vec::new();
    file.take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| format!("read failed: {error}"))?;
    if bytes.len() as u64 > limit {
        return Err("file size limit exceeded".into());
    }
    String::from_utf8(bytes).map_err(|_| "file is not UTF-8".into())
}

fn options(case: &dat::Case, scripting_enabled: bool) -> Result<ParseOptions, String> {
    let fragment = case
        .fields
        .get("document-fragment")
        .map(|context| {
            let (namespace, name) = match context.split_once(' ') {
                None => (Namespace::Html, context.as_str()),
                Some(("svg", name)) => (Namespace::Svg, name),
                Some(("math" | "mathml", name)) => (Namespace::MathMl, name),
                Some(("html", name)) => (Namespace::Html, name),
                _ => return Err("unsupported fragment namespace".to_owned()),
            };
            Ok(FragmentContext {
                namespace,
                local_name: name.into(),
            })
        })
        .transpose()?;
    Ok(ParseOptions {
        scripting_enabled,
        fragment,
    })
}

fn child(args: &[OsString]) -> Result<(), String> {
    if args.len() != 4 {
        return Err("invalid internal case arguments".into());
    }
    let cases = dat::parse(&read_text(Path::new(&args[0]), MAX_DAT_BYTES)?)?;
    let index = args[1]
        .to_str()
        .ok_or("invalid case index")?
        .parse::<usize>()
        .map_err(|_| "invalid case index")?;
    let scripting = match args[2].to_str() {
        Some("on") => true,
        Some("off") => false,
        _ => return Err("invalid script flag".into()),
    };
    let case = cases.get(index).ok_or("case index out of range")?;
    let parsed = Document::parse_with_options(&case.fields["data"], &options(case, scripting)?);
    let result = match parsed.and_then(|document| document.tree_dump()) {
        Ok(tree) => format!("OK\n{tree}"),
        Err(error) => format!("ERROR\n{error}"),
    };
    // The parent creates a private run directory and removes only this exact result
    // file. create_new prevents a stale result from masquerading as a fresh reply.
    let mut output = File::create_new(Path::new(&args[3]))
        .map_err(|error| format!("result creation failed: {error}"))?;
    output
        .write_all(result.as_bytes())
        .map_err(|error| format!("result write failed: {error}"))
}

struct Scratch(PathBuf);

impl Scratch {
    fn new(base: &Path) -> Result<Self, String> {
        fs::create_dir_all(base)
            .map_err(|error| format!("scratch base creation failed: {error}"))?;
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| "clock before epoch")?
            .as_nanos();
        let path = base.join(format!("opensequoia-tree-{}-{stamp}", std::process::id()));
        fs::create_dir(&path).map_err(|error| format!("scratch creation failed: {error}"))?;
        Ok(Self(path))
    }
    fn result(&self) -> PathBuf {
        self.0.join("result.txt")
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_file(self.result());
        let _ = fs::remove_dir(&self.0); // Not a recursive deletion; only the owned file is removed.
    }
}

fn execute(
    file: &Path,
    index: usize,
    scripting: bool,
    scratch: &Scratch,
    timeout: Duration,
) -> Result<String, (&'static str, String)> {
    let executable =
        std::env::current_exe().map_err(|error| ("harness_error", error.to_string()))?;
    let mut command = Command::new(executable);
    command
        .arg("--case")
        .arg(file)
        .arg(index.to_string())
        .arg(if scripting { "on" } else { "off" })
        .arg(scratch.result());
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // Documented CREATE_NO_WINDOW; test subprocesses need no interactive console.
        command.creation_flags(0x0800_0000);
    }
    let mut child = command
        .spawn()
        .map_err(|error| ("harness_error", format!("spawn failed: {error}")))?;
    let start = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if start.elapsed() < timeout => std::thread::sleep(Duration::from_millis(2)),
            result => {
                let cleanup = child.kill().and_then(|()| child.wait().map(|_| ()));
                let _ = fs::remove_file(scratch.result());
                if let Err(error) = cleanup {
                    return Err(("harness_error", format!("child cleanup failed: {error}")));
                }
                return match result {
                    Ok(None) => Err((
                        "timeout",
                        "case exceeded the recorded subprocess deadline".into(),
                    )),
                    Err(error) => Err(("harness_error", format!("child wait failed: {error}"))),
                    Ok(Some(_)) => unreachable!(),
                };
            }
        }
    };
    if !status.success() {
        let _ = fs::remove_file(scratch.result());
        return Err((
            if status.code() == Some(2) {
                "harness_error"
            } else {
                "crash"
            },
            format!("child exited {status}"),
        ));
    }
    let result = read_text(&scratch.result(), MAX_TREE_DUMP_BYTES as u64 + 1024)
        .map_err(|error| ("harness_error", error));
    fs::remove_file(scratch.result())
        .map_err(|error| ("harness_error", format!("result cleanup failed: {error}")))?;
    result
}

fn json(value: &str) -> String {
    let mut output = String::from("\"");
    for ch in value.chars() {
        match ch {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            ch if ch.is_control() => output.push_str(&format!("\\u{:04x}", ch as u32)),
            ch => output.push(ch),
        }
    }
    output.push('"');
    output
}

fn mismatch(expected: &str, actual: &str) -> String {
    let offset = expected
        .bytes()
        .zip(actual.bytes())
        .position(|(a, b)| a != b)
        .unwrap_or(expected.len().min(actual.len()));
    // Preserve case identity and exact offset; bounded excerpts cannot stand in for
    // the whole tree. --case reproduces the complete output for any failing input.
    let excerpt = |text: &str| {
        let mut start = offset.min(text.len());
        while !text.is_char_boundary(start) {
            start -= 1;
        }
        text[start..].chars().take(160).collect::<String>()
    };
    format!(
        "tree mismatch at byte {offset}; expected excerpt {}; actual excerpt {}",
        json(&excerpt(expected)),
        json(&excerpt(actual))
    )
}

fn parent(args: &[OsString]) -> Result<bool, String> {
    let mut suite = None;
    let mut report = None;
    let mut revision = None;
    let mut scratch_dir = None;
    let mut timeout_ms = 5000_u64;
    if !args.len().is_multiple_of(2) {
        return Err("options need name/value pairs".into());
    }
    for pair in args.as_chunks::<2>().0 {
        match pair[0].to_str() {
            Some("--suite") if suite.is_none() => suite = Some(PathBuf::from(&pair[1])),
            Some("--report") if report.is_none() => report = Some(PathBuf::from(&pair[1])),
            Some("--suite-revision") if revision.is_none() => {
                revision = Some(pair[1].to_str().ok_or("revision must be UTF-8")?.to_owned())
            }
            Some("--scratch-dir") if scratch_dir.is_none() => {
                scratch_dir = Some(PathBuf::from(&pair[1]))
            }
            Some("--timeout-ms") => {
                timeout_ms = pair[1]
                    .to_str()
                    .ok_or("invalid deadline")?
                    .parse()
                    .map_err(|_| "invalid deadline")?
            }
            _ => return Err("unknown or duplicate option".into()),
        }
    }
    let suite = suite
        .ok_or("missing --suite")?
        .canonicalize()
        .map_err(|error| format!("suite path failed: {error}"))?;
    let revision = revision.ok_or("missing --suite-revision")?;
    if revision.len() != 40
        || !revision.bytes().all(|byte| byte.is_ascii_hexdigit())
        || !(1..=60_000).contains(&timeout_ms)
    {
        return Err("invalid suite revision or timeout".into());
    }
    let mut files: Vec<_> = fs::read_dir(&suite)
        .map_err(|error| error.to_string())?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<_, _>>()
        .map_err(|error| error.to_string())?;
    files.retain(|path| path.extension().is_some_and(|extension| extension == "dat"));
    files.sort();
    if files.is_empty() || files.len() > 10_000 {
        return Err("invalid .dat file count".into());
    }
    let scratch = Scratch::new(
        scratch_dir
            .as_deref()
            .unwrap_or_else(|| Path::new("target/tree3")),
    )?;
    let mut output = BufWriter::new(
        File::create(report.ok_or("missing --report")?).map_err(|error| error.to_string())?,
    );
    writeln!(output, "{{\"kind\":\"metadata\",\"schema\":1,\"engine_version\":{},\"suite\":{},\"suite_revision_label\":{},\"files\":{},\"timeout_ms\":{},\"limits\":\"tree assertions only; parser diagnostics not compared; no JS execution; source label verified by separate SHA256 inventory\"}}",
        json(env!("CARGO_PKG_VERSION")), json(&suite.to_string_lossy()), json(&revision), files.len(), timeout_ms).map_err(|error| error.to_string())?;
    let mut counts = std::collections::BTreeMap::from([
        ("pass", 0),
        ("fail", 0),
        ("unsupported", 0),
        ("skip", 0),
        ("timeout", 0),
        ("crash", 0),
        ("harness_error", 0),
    ]);
    let mut total_cases = 0;
    for file in files {
        let name = file
            .file_name()
            .ok_or("invalid file name")?
            .to_string_lossy();
        let cases = match read_text(&file, MAX_DAT_BYTES).and_then(|source| dat::parse(&source)) {
            Ok(cases) => cases,
            Err(error) => {
                *counts
                    .get_mut("harness_error")
                    .ok_or("missing outcome counter")? += 1;
                writeln!(output, "{{\"kind\":\"file_error\",\"file\":{},\"status\":\"harness_error\",\"detail\":{}}}", json(&name), json(&error)).map_err(|error| error.to_string())?;
                continue;
            }
        };
        total_cases += cases.len();
        for (index, case) in cases.iter().enumerate() {
            let modes: &[bool] = if case.fields.contains_key("script-on") {
                &[true]
            } else if case.fields.contains_key("script-off") {
                &[false]
            } else {
                &[false, true]
            };
            for &scripting in modes {
                let (status, detail) = if name.starts_with("scripted_") {
                    (
                        "unsupported",
                        "upstream scripted_* cases require executing JavaScript during parsing"
                            .to_owned(),
                    )
                } else {
                    match execute(
                        &file,
                        index,
                        scripting,
                        &scratch,
                        Duration::from_millis(timeout_ms),
                    ) {
                        Ok(reply) => match reply.strip_prefix("OK\n") {
                            Some(actual) if actual == case.fields["document"] => {
                                ("pass", String::new())
                            }
                            Some(actual) => ("fail", mismatch(&case.fields["document"], actual)),
                            None => match reply.strip_prefix("ERROR\n") {
                                Some(error) => ("fail", format!("engine rejected case: {error}")),
                                None => ("harness_error", "invalid child reply".into()),
                            },
                        },
                        Err(result) => result,
                    }
                };
                *counts.get_mut(status).ok_or("missing outcome counter")? += 1;
                writeln!(output, "{{\"kind\":\"case\",\"id\":{},\"file\":{},\"index\":{},\"scripting\":{},\"fragment\":{},\"status\":{},\"detail\":{}}}",
                    json(&format!("{name}#{index}@{}", if scripting { "on" } else { "off" })), json(&name), index, scripting,
                    case.fields.get("document-fragment").map_or("null".to_owned(), |fragment| json(fragment)), json(status), json(&detail)).map_err(|error| error.to_string())?;
                output.flush().map_err(|error| error.to_string())?;
            }
        }
    }
    let summary = counts
        .iter()
        .map(|(status, count)| format!("{}:{count}", json(status)))
        .collect::<Vec<_>>()
        .join(",");
    writeln!(
        output,
        "{{\"kind\":\"summary\",\"source_cases\":{total_cases},\"outcomes\":{{{summary}}}}}"
    )
    .map_err(|error| error.to_string())?;
    output.flush().map_err(|error| error.to_string())?;
    println!("TREE_CONSTRUCTION_REPORT cases={total_cases} {{{summary}}}");
    Ok(counts
        .iter()
        .all(|(status, count)| *status == "pass" || *count == 0))
}

fn main() -> ExitCode {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.first().is_some_and(|arg| arg == "--case") {
        return match child(&args[1..]) {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("case harness: {error}");
                ExitCode::from(2)
            }
        };
    }
    match parent(&args) {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(error) => {
            eprintln!("tree harness: {error}");
            ExitCode::from(2)
        }
    }
}
