//! Opt-in local development input boundary; direct URI/share/device paths and
//! observed reparse points are rejected. Ancestor checks have a TOCTOU limit and
//! are not a filesystem/network sandbox. Default fixture performs no file open.

use std::{
    ffi::OsString,
    fs::OpenOptions,
    io::Read,
    path::{Component, Path, PathBuf},
};

use opensequoia_engine::MAX_INPUT_BYTES;

#[derive(Debug, PartialEq, Eq)]
pub enum Mode {
    Window { smoke_test: bool },
    Inspect,
    ProcessSmoke,
    AppSmoke,
    SandboxSmoke,
    SandboxDenialWorker { port: u16 },
    RendererWorker,
    Help,
    Version,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Options {
    pub mode: Mode,
    pub path: Option<PathBuf>,
}

pub fn options(args: impl IntoIterator<Item = OsString>) -> Result<Options, String> {
    let mut args = args.into_iter().peekable();
    if args
        .peek()
        .is_some_and(|arg| arg == "--sandbox-denial-worker")
    {
        args.next();
        return sandbox_denial_options(args);
    }
    let mut mode = Mode::Window { smoke_test: false };
    let mut mode_set = false;
    let mut path = None;
    let mut positional_only = false;
    for value in args {
        let value = value
            .into_string()
            .map_err(|_| "arguments must contain valid Unicode".to_owned())?;
        if !positional_only && value == "--" {
            positional_only = true;
            continue;
        }
        let selected = if positional_only {
            None
        } else {
            match value.as_str() {
                "--inspect" => Some(Mode::Inspect),
                "--smoke-test" => Some(Mode::Window { smoke_test: true }),
                "--process-smoke" => Some(Mode::ProcessSmoke),
                "--app-smoke" => Some(Mode::AppSmoke),
                "--sandbox-smoke" => Some(Mode::SandboxSmoke),
                "--renderer-worker" => Some(Mode::RendererWorker),
                "--help" | "-h" => Some(Mode::Help),
                "--version" | "-V" => Some(Mode::Version),
                _ => None,
            }
        };
        if let Some(selected) = selected {
            if mode_set || path.is_some() {
                return Err("choose one mode before the optional local filename".to_owned());
            }
            mode = selected;
            mode_set = true;
        } else {
            if !positional_only && value.starts_with('-') {
                return Err("unknown option; use --help".to_owned());
            }
            if path.is_some()
                || matches!(
                    mode,
                    Mode::Help
                        | Mode::Version
                        | Mode::ProcessSmoke
                        | Mode::AppSmoke
                        | Mode::SandboxSmoke
                        | Mode::RendererWorker
                )
            {
                return Err("unexpected extra argument".to_owned());
            }
            check_local_path(&value)?;
            path = Some(PathBuf::from(value));
        }
    }
    Ok(Options { mode, path })
}

// Internal test helper: exact local arguments, never a remote endpoint selector.
fn sandbox_denial_options(mut args: impl Iterator<Item = OsString>) -> Result<Options, String> {
    let path = args
        .next()
        .ok_or("missing sandbox probe filename")?
        .into_string()
        .map_err(|_| "sandbox probe filename must contain valid Unicode")?;
    check_local_path(&path)?;
    if path.len() > 4096 || !Path::new(&path).is_absolute() {
        return Err("sandbox probe requires a bounded absolute local filename".to_owned());
    }
    let port = args
        .next()
        .ok_or("missing sandbox loopback port")?
        .into_string()
        .map_err(|_| "sandbox loopback port must be decimal")?;
    if port.is_empty() || port.len() > 5 || !port.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err("sandbox loopback port must be decimal".to_owned());
    }
    let port: u16 = port
        .parse()
        .map_err(|_| "sandbox loopback port is out of range")?;
    if port == 0 || args.next().is_some() {
        return Err(
            "sandbox probe requires exactly a filename and nonzero loopback port".to_owned(),
        );
    }
    Ok(Options {
        mode: Mode::SandboxDenialWorker { port },
        path: Some(path.into()),
    })
}

pub(crate) fn check_local_path(path: &str) -> Result<(), String> {
    let bytes = path.as_bytes();
    let drive_absolute = bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && matches!(bytes[2], b'/' | b'\\');
    if path.is_empty()
        || path.contains('\0')
        || path.starts_with("\\\\")
        || path.starts_with("//")
        || path
            .char_indices()
            .any(|(i, c)| c == ':' && !(drive_absolute && i == 1))
    {
        return Err(
            "only local regular filenames are supported; URIs, shares and devices are rejected"
                .to_owned(),
        );
    }
    for part in path.split(['/', '\\']) {
        let base = part
            .trim_end_matches([' ', '.'])
            .split('.')
            .next()
            .unwrap_or("")
            .trim_end_matches(' ');
        let name = base.to_ascii_uppercase();
        let serial_device = name
            .strip_prefix("COM")
            .or_else(|| name.strip_prefix("LPT"))
            .is_some_and(|number| {
                matches!(
                    number,
                    "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" | "¹" | "²" | "³"
                )
            });
        if matches!(
            name.as_str(),
            "CON" | "PRN" | "AUX" | "NUL" | "CONIN$" | "CONOUT$"
        ) || serial_device
        {
            return Err("Windows device filenames are rejected".to_owned());
        }
    }
    Ok(())
}

pub fn load(path: Option<&PathBuf>) -> Result<String, String> {
    let Some(path) = path else {
        return Ok(include_str!("../../../fixtures/welcome.html").to_owned());
    };
    check_local_path(
        path.to_str()
            .ok_or("local filename must contain valid Unicode")?,
    )?;
    reject_reparse_ancestors(path)?;
    let preflight = std::fs::symlink_metadata(path)
        .map_err(|error| format!("cannot inspect local file: {}", error.kind()))?;
    if !preflight.is_file() || is_reparse(&preflight) {
        return Err("input must be a regular file".to_owned());
    }
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        // FILE_FLAG_OPEN_REPARSE_POINT prevents final-component link traversal.
        // Directory ancestors can still change after checks; this is not isolation.
        options.custom_flags(0x0020_0000);
    }
    let file = options
        .open(path)
        .map_err(|error| format!("cannot open local file: {}", error.kind()))?;
    let metadata = file
        .metadata()
        .map_err(|error| format!("cannot inspect local file: {}", error.kind()))?;
    if !metadata.is_file() || is_reparse(&metadata) {
        return Err("input must be a regular file".to_owned());
    }
    if metadata.len() > MAX_INPUT_BYTES as u64 {
        return Err("input exceeds the 256 KiB development limit".to_owned());
    }
    let mut bytes = Vec::new();
    file.take(MAX_INPUT_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| format!("cannot read local file: {}", error.kind()))?;
    if bytes.len() > MAX_INPUT_BYTES {
        return Err("input exceeds the 256 KiB development limit".to_owned());
    }
    String::from_utf8(bytes).map_err(|_| "input must be UTF-8 HTML".to_owned())
}

fn reject_reparse_ancestors(path: &Path) -> Result<(), String> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|_| "cannot resolve local directory")?
            .join(path)
    };
    #[cfg(windows)]
    crate::windows::reject_nonlocal_volume(&absolute)?;
    let mut prefix = PathBuf::new();
    for component in absolute.components() {
        match component {
            Component::Prefix(_) | Component::RootDir | Component::Normal(_) => {
                prefix.push(component)
            }
            Component::CurDir => continue,
            Component::ParentDir => {
                prefix.pop();
            }
        }
        // A Windows drive prefix alone is not a complete path; inspect after root.
        if matches!(component, Component::Prefix(_)) {
            continue;
        }
        let metadata = std::fs::symlink_metadata(&prefix)
            .map_err(|error| format!("cannot validate local path: {}", error.kind()))?;
        if metadata.file_type().is_symlink() || is_reparse(&metadata) {
            return Err(
                "local development input cannot traverse reparse points or symbolic links"
                    .to_owned(),
            );
        }
    }
    Ok(())
}

#[cfg(windows)]
fn is_reparse(metadata: &std::fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;
    metadata.file_attributes() & 0x0400 != 0 // FILE_ATTRIBUTE_REPARSE_POINT
}

#[cfg(not(windows))]
fn is_reparse(metadata: &std::fs::Metadata) -> bool {
    metadata.file_type().is_symlink()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);

    struct Fixture {
        directory: PathBuf,
        path: PathBuf,
    }

    impl Fixture {
        fn new(bytes: &[u8]) -> Self {
            let directory = std::env::temp_dir().join(format!(
                "opensequoia-input-{}-{}",
                std::process::id(),
                NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
            ));
            std::fs::create_dir(&directory).unwrap();
            let path = directory.join("input.html");
            std::fs::write(&path, bytes).unwrap();
            Self { directory, path }
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.path);
            let _ = std::fs::remove_dir(&self.directory);
        }
    }

    fn parse(args: &[&str]) -> Result<Options, String> {
        options(args.iter().map(OsString::from))
    }

    #[test]
    fn defaults_to_bundled_local_document() {
        assert_eq!(
            parse(&[]).unwrap(),
            Options {
                mode: Mode::Window { smoke_test: false },
                path: None
            }
        );
        assert!(load(None).unwrap().contains("OpenSequoia"));
    }

    #[test]
    fn modes_are_explicit_and_exclusive() {
        assert_eq!(
            parse(&["--inspect", "sample.html"]).unwrap().mode,
            Mode::Inspect
        );
        assert_eq!(
            parse(&["--smoke-test"]).unwrap().mode,
            Mode::Window { smoke_test: true }
        );
        assert!(parse(&["--inspect", "--smoke-test"]).is_err());
        assert!(parse(&["--help", "sample.html"]).is_err());
        assert!(parse(&["sample.html", "--inspect"]).is_err());
        assert!(parse(&["--unknown"]).is_err());
        assert!(parse(&["--renderer-worker", "sample.html"]).is_err());
        assert!(parse(&["--process-smoke", "sample.html"]).is_err());
        assert!(parse(&["a.html", "b.html"]).is_err());
        assert!(parse(&["--", "-sample.html"]).is_ok());
    }

    #[test]
    fn sandbox_probes_have_exact_local_arguments() {
        assert!(parse(&["--sandbox-smoke"]).is_ok());
        assert!(
            parse(&[
                "--sandbox-denial-worker",
                "C:/tmp/opensequoia-probe/denied.bin",
                "65535"
            ])
            .is_ok()
        );
        for args in [
            vec!["--sandbox-smoke", "sample.html"],
            vec!["--sandbox-smoke", "--inspect"],
            vec!["--sandbox-denial-worker"],
            vec!["--sandbox-denial-worker", "C:/tmp/denied.bin"],
            vec!["--sandbox-denial-worker", "relative.bin", "1234"],
            vec!["--sandbox-denial-worker", "https://example.com", "1234"],
            vec!["--sandbox-denial-worker", "C:/tmp/denied.bin", "0"],
            vec!["--sandbox-denial-worker", "C:/tmp/denied.bin", "65536"],
            vec!["--sandbox-denial-worker", "C:/tmp/denied.bin", "+80"],
            vec!["--sandbox-denial-worker", "C:/tmp/denied.bin", " 80"],
            vec![
                "--sandbox-denial-worker",
                "C:/tmp/denied.bin",
                "80",
                "extra",
            ],
            vec![
                "--inspect",
                "--sandbox-denial-worker",
                "C:/tmp/denied.bin",
                "80",
            ],
        ] {
            assert!(parse(&args).is_err(), "accepted {args:?}");
        }
    }

    #[test]
    fn refuses_network_and_device_inputs() {
        for path in [
            "https://example.com",
            "file:///tmp/a",
            "data:text/html,x",
            "C:relative.html",
            "a.html:stream",
            "\\\\host\\share\\a.html",
            "//host/a",
            "\\\\?\\C:\\a.html",
            "NUL",
            "CON.txt",
            "NUL .txt",
            "COM¹.html",
            "CONIN$",
            "C:\\a\\COM1.html",
            "LPT9.",
        ] {
            assert!(parse(&[path]).is_err(), "accepted {path}");
        }
        for path in [
            "a.html",
            "fixtures/welcome.html",
            "C:\\tmp\\a.html",
            "C:/tmp/a.html",
            "café.html",
        ] {
            assert!(parse(&[path]).is_ok(), "rejected {path}");
        }
    }

    #[test]
    fn loader_validates_real_utf8_and_file_limits() {
        let unicode = Fixture::new("<p>café 🌲</p>".as_bytes());
        assert_eq!(load(Some(&unicode.path)).unwrap(), "<p>café 🌲</p>");
        let invalid = Fixture::new(&[0xff, 0xfe]);
        assert!(load(Some(&invalid.path)).unwrap_err().contains("UTF-8"));
        let exact = Fixture::new(&vec![b'x'; MAX_INPUT_BYTES]);
        assert_eq!(load(Some(&exact.path)).unwrap().len(), MAX_INPUT_BYTES);
        let oversized = Fixture::new(&vec![b'x'; MAX_INPUT_BYTES + 1]);
        assert!(load(Some(&oversized.path)).unwrap_err().contains("256 KiB"));
        assert!(
            load(Some(&unicode.directory))
                .unwrap_err()
                .contains("regular file")
        );
    }

    #[test]
    #[cfg(windows)]
    fn loader_rejects_a_real_local_directory_junction() {
        use std::os::windows::process::CommandExt;

        let input = Fixture::new(b"<p>junction target</p>");
        let holder = Fixture::new(b"holder");
        let junction = holder.directory.join("junction");
        let status = std::process::Command::new("powershell.exe")
            .args(["-NoProfile", "-NonInteractive", "-Command",
                "New-Item -ItemType Junction -Path $env:OPENSEQUOIA_TEST_LINK -Target $env:OPENSEQUOIA_TEST_TARGET -ErrorAction Stop | Out-Null"])
            .env("OPENSEQUOIA_TEST_LINK", &junction)
            .env("OPENSEQUOIA_TEST_TARGET", &input.directory)
            .creation_flags(0x0800_0000) // CREATE_NO_WINDOW; test-only helper
            .status().unwrap();
        assert!(
            status.success(),
            "could not create the local junction test fixture"
        );
        let result = load(Some(&junction.join("input.html")));
        // Windows remove_dir/RemoveDirectory removes the junction itself, not its target.
        std::fs::remove_dir(&junction).unwrap();
        assert!(result.unwrap_err().contains("reparse points"));
        assert_eq!(
            std::fs::read(&input.path).unwrap(),
            b"<p>junction target</p>"
        );
    }
}
