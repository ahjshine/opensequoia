#![deny(unsafe_op_in_unsafe_fn)]
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod browser;
#[cfg(windows)]
mod chrome;
mod controller;
mod input;
#[cfg(windows)]
mod windows;
#[cfg(windows)]
mod windows_process;
#[cfg(windows)]
mod windows_sandbox;
mod worker;

use input::Mode;
use opensequoia_engine::{Diagnostic, Document};

fn main() -> std::process::ExitCode {
    match run() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("OpenSequoia: {error}");
            std::process::ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let options = input::options(std::env::args_os().skip(1))?;
    match options.mode {
        Mode::Help => {
            println!(
                "OpenSequoia {} — local HTML rendering development increment",
                env!("CARGO_PKG_VERSION")
            );
            println!("Usage: opensequoia [--inspect | --smoke-test] [local.html]");
            println!(
                "Default input: bundled welcome fixture. Local files: UTF-8, at most 256 KiB."
            );
            println!(
                "Window: Ctrl+L address, Ctrl+T/W tabs, Ctrl+Tab switch, Alt+Left/Right history, F5 reload."
            );
            println!("--inspect uses the same Rust parse/layout path without a window.");
            println!(
                "--smoke-test requires a successful native paint and then closes automatically."
            );
            println!(
                "--process-smoke verifies a local child renderer, protocol rejection and restart."
            );
            println!("--app-smoke verifies native controls, local navigation and child rendering.");
            println!(
                "--sandbox-smoke runs opt-in AppContainer renderer/denial probes with synthetic local data."
            );
            println!(
                "No remote browsing, scripts, profiles or full website compatibility is implemented."
            );
            return Ok(());
        }
        Mode::Version => {
            println!("OpenSequoia {}", env!("CARGO_PKG_VERSION"));
            return Ok(());
        }
        Mode::RendererWorker => return worker::renderer_worker(),
        Mode::ProcessSmoke => return worker::process_smoke(),
        Mode::SandboxSmoke => {
            #[cfg(windows)]
            {
                return worker::sandbox_smoke();
            }
            #[cfg(not(windows))]
            {
                return Err("sandbox smoke verification requires Windows".to_owned());
            }
        }
        Mode::SandboxDenialWorker { port } => {
            #[cfg(windows)]
            {
                let path = options.path.ok_or("missing sandbox probe filename")?;
                return windows_sandbox::denial_worker(&path, port)
                    .map_err(|error| format!("sandbox denial helper failed: {error}"));
            }
            #[cfg(not(windows))]
            {
                let _ = port;
                return Err("AppContainer verification requires Windows".to_owned());
            }
        }
        Mode::AppSmoke => {
            #[cfg(windows)]
            {
                return windows::app_smoke();
            }
            #[cfg(not(windows))]
            {
                return Err("native application verification requires Windows".to_owned());
            }
        }
        Mode::Inspect | Mode::Window { .. } => {}
    }
    if options.mode == Mode::Inspect {
        let html = input::load(options.path.as_ref())?;
        let document =
            Document::parse(&html).map_err(|error| format!("HTML parse failed: {error}"))?;
        let display = document
            .layout(960)
            .map_err(|error| format!("layout failed: {error}"))?;
        println!("OpenSequoia local HTML inspection");
        println!(
            "nodes={} viewport_width={} content_height={} text_runs={}",
            document.node_count(),
            display.width,
            display.height,
            display.items.len()
        );
        let mut parse_errors = 0;
        let mut unsupported_elements = 0;
        let mut unsupported_css = 0;
        let mut unsupported_features = 0;
        for diagnostic in document.diagnostics() {
            match diagnostic {
                Diagnostic::HtmlParseError(_) => parse_errors += 1,
                Diagnostic::UnsupportedElement(_) => unsupported_elements += 1,
                Diagnostic::UnsupportedCss => unsupported_css += 1,
                Diagnostic::UnsupportedFeature(_) => unsupported_features += 1,
            }
        }
        println!(
            "diagnostics={} html_recovery={} unsupported_elements={} unsupported_css={} unsupported_features={}",
            document.diagnostics().len(),
            parse_errors,
            unsupported_elements,
            unsupported_css,
            unsupported_features
        );
        println!("Limited static layout; no script execution or website compatibility claim.");
        return Ok(());
    }
    let Mode::Window { smoke_test } = options.mode else {
        return Err("invalid window mode".to_owned());
    };
    #[cfg(windows)]
    {
        let source = options
            .path
            .map(browser::PageSource::Local)
            .unwrap_or(browser::PageSource::Welcome);
        windows::run(source, smoke_test)
    }
    #[cfg(not(windows))]
    {
        let _ = (options.path, smoke_test);
        Err("the native application currently requires Windows; use --inspect for engine verification".to_owned())
    }
}
