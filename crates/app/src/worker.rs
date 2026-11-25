#![forbid(unsafe_code)]

//! Bounded local child-renderer lifecycle probe. This is not a sandbox, an
//! authenticated peer protocol, site isolation or native-window integration.

use std::{
    io::{self, Read, Write},
    process::{Command, ExitStatus},
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

use opensequoia_engine::{DisplayList, Document};
use opensequoia_ipc::{ErrorCode, Message, encode, read_message, write_message};

const EXCHANGE_TIMEOUT: Duration = Duration::from_secs(5);
const REAP_TIMEOUT: Duration = Duration::from_secs(1);

#[cfg(windows)]
type Child = crate::windows_process::ContainedChild;
#[cfg(not(windows))]
type Child = std::process::Child;

fn join_io(io_thread: thread::JoinHandle<()>) -> Result<(), String> {
    let deadline = Instant::now() + REAP_TIMEOUT;
    while !io_thread.is_finished() {
        #[cfg(windows)]
        crate::windows_process::cancel_io(&io_thread)
            .map_err(|error| format!("renderer I/O cancellation failed: {error}"))?;
        if Instant::now() >= deadline {
            // Do not turn failed I/O cancellation into an unbounded parent wait.
            // Dropping detaches this thread; the probe fails and claims no cleanup
            // completeness. Production job-tree/cancelled-I/O policy remains work.
            return Err("renderer I/O did not stop after child cleanup".to_owned());
        }
        thread::sleep(Duration::from_millis(5));
    }
    io_thread
        .join()
        .map_err(|_| "child I/O thread panicked".to_owned())
}

pub fn renderer_worker() -> Result<(), String> {
    serve(&mut io::stdin().lock(), &mut io::stdout().lock())
}

/// Parse/layout only in the bounded child renderer. No in-process fallback.
pub fn render(html: &str, width: u32) -> Result<DisplayList, String> {
    if html.len() > opensequoia_engine::MAX_INPUT_BYTES {
        return Err("renderer input exceeds the 256 KiB limit".to_owned());
    }
    let frame = encode(&Message::Render {
        html: html.to_owned(),
        width,
    })
    .map_err(|error| format!("renderer request rejected: {error}"))?;
    let result = exchange(child_command()?, frame, EXCHANGE_TIMEOUT)?;
    validate_reply(result.reply, result.status.success(), width)
}

fn validate_reply(
    reply: Message,
    successful_exit: bool,
    requested_width: u32,
) -> Result<DisplayList, String> {
    match reply {
        Message::Display(display) if successful_exit && display.width == requested_width => {
            Ok(display)
        }
        Message::Display(_) if successful_exit => {
            Err("renderer reply viewport did not match the request".to_owned())
        }
        Message::Error { message, .. } => Err(message),
        _ => Err("renderer reply did not match successful process exit".to_owned()),
    }
}

fn serve(reader: &mut impl Read, writer: &mut impl Write) -> Result<(), String> {
    let request = match read_message(reader) {
        Ok(Message::Render { html, width }) => (html, width),
        Ok(_) | Err(_) => {
            let error = Message::Error {
                code: ErrorCode::Protocol,
                message: "invalid renderer request".to_owned(),
            };
            write_message(writer, &error)
                .map_err(|error| format!("worker error reply failed: {error}"))?;
            writer
                .flush()
                .map_err(|error| format!("worker reply flush failed: {error}"))?;
            return Err("renderer protocol rejected".to_owned());
        }
    };
    let result = Document::parse(&request.0)
        .map_err(|_| ErrorCode::Parse)
        .and_then(|document| document.layout(request.1).map_err(|_| ErrorCode::Layout));
    let (reply, failed) = match result {
        Ok(display) => (Message::Display(display), false),
        Err(code) => (
            Message::Error {
                code,
                message: "document render rejected".to_owned(),
            },
            true,
        ),
    };
    write_message(writer, &reply).map_err(|error| format!("worker reply failed: {error}"))?;
    writer
        .flush()
        .map_err(|error| format!("worker reply flush failed: {error}"))?;
    if failed {
        Err("renderer rejected document".to_owned())
    } else {
        Ok(())
    }
}

struct ManagedChild {
    child: Child,
    reaped: bool,
}

impl ManagedChild {
    fn until_exit(&mut self, deadline: Instant) -> Result<ExitStatus, String> {
        loop {
            if let Some(status) = self
                .child
                .try_wait()
                .map_err(|error| format!("child status failed: {error}"))?
            {
                self.reaped = true;
                return Ok(status);
            }
            if Instant::now() >= deadline {
                return Err("renderer child exit timed out".to_owned());
            }
            thread::sleep(Duration::from_millis(5));
        }
    }

    fn terminate(&mut self) -> Result<ExitStatus, String> {
        // try_wait first handles a normal-exit/kill race. A successful returned
        // status proves reaping; unsuccessful kill/reap is an explicit failure.
        if let Some(status) = self
            .child
            .try_wait()
            .map_err(|error| format!("child status failed: {error}"))?
        {
            self.reaped = true;
            return Ok(status);
        }
        let kill = self.child.kill();
        self.until_exit(Instant::now() + REAP_TIMEOUT)
            .map_err(|error| match kill {
                Ok(()) => format!("cannot establish child cleanup: {error}"),
                Err(kill_error) => {
                    format!("cannot establish child cleanup: kill failed ({kill_error}); {error}")
                }
            })
    }
}

impl Drop for ManagedChild {
    fn drop(&mut self) {
        if !self.reaped {
            // Last-resort bounded error/panic cleanup. Never counted as a verified
            // reap: normal/error paths explicitly call terminate/until_exit.
            let _ = self.terminate();
        }
    }
}

struct Exchange {
    reply: Message,
    status: ExitStatus,
}

#[cfg(windows)]
struct OutputExchange {
    output: String,
    status: ExitStatus,
}

fn child_command() -> Result<Command, String> {
    let executable = std::env::current_exe()
        .map_err(|error| format!("cannot locate own executable: {error}"))?;
    let mut command = Command::new(&executable);
    command.arg("--renderer-worker");
    command.env_clear();
    // Keep only OS loader context, not inherited authentication/service secrets.
    for name in ["SystemRoot", "WINDIR"] {
        if let Some(value) = std::env::var_os(name) {
            command.env(name, value);
        }
    }
    if let Some(directory) = executable.parent() {
        command.current_dir(directory);
    }
    Ok(command)
}

fn exchange(command: Command, frame: Vec<u8>, timeout: Duration) -> Result<Exchange, String> {
    #[cfg(windows)]
    let child = crate::windows_process::spawn(&command)
        .map_err(|error| format!("renderer child launch failed: {error}"))?;
    #[cfg(not(windows))]
    let child = {
        let mut command = command;
        command
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null());
        command
            .spawn()
            .map_err(|error| format!("renderer child launch failed: {error}"))?
    };
    exchange_child(child, frame, timeout)
}

fn exchange_child(child: Child, frame: Vec<u8>, timeout: Duration) -> Result<Exchange, String> {
    let mut child = ManagedChild {
        child,
        reaped: false,
    };
    let mut stdin = child
        .child
        .stdin
        .take()
        .ok_or("child input pipe unavailable")?;
    let mut stdout = child
        .child
        .stdout
        .take()
        .ok_or("child output pipe unavailable")?;
    let (sender, receiver) = mpsc::sync_channel(1);
    // Concurrent I/O ensures a child that refuses stdin or never produces stdout
    // cannot block the supervisor before its deadline. IPC reader bounds allocation.
    let io_thread = thread::Builder::new()
        .name("renderer-probe-io".to_owned())
        .spawn(move || {
            let result = (|| {
                stdin
                    .write_all(&frame)
                    .map_err(|error| format!("child request failed: {error}"))?;
                drop(stdin); // EOF after one request; no arbitrary inherited input stream.
                let reply = read_message(&mut stdout)
                    .map_err(|error| format!("child reply rejected: {error}"))?;
                let mut trailing = [0u8; 1];
                if stdout
                    .read(&mut trailing)
                    .map_err(|error| format!("child reply EOF failed: {error}"))?
                    != 0
                {
                    return Err("child sent trailing protocol data".to_owned());
                }
                Ok(reply)
            })();
            let _ = sender.send(result);
        })
        .map_err(|error| format!("child I/O thread failed: {error}"))?;
    let deadline = Instant::now() + timeout;
    let received = receiver.recv_timeout(timeout);
    let status = match &received {
        Ok(_) => match child.until_exit(deadline) {
            Ok(status) => status,
            Err(error) => {
                child.terminate()?;
                join_io(io_thread)?;
                return Err(error);
            }
        },
        Err(_) => {
            child.terminate()?;
            join_io(io_thread)?;
            return Err("renderer exchange timed out or disconnected; child reaped".to_owned());
        }
    };
    join_io(io_thread)?;
    let reply = received.map_err(|_| "renderer exchange unavailable")??;
    Ok(Exchange { reply, status })
}

/// Bounded non-IPC output for the real denial helper. Exit, output, timeout and
/// cleanup remain separate outcomes, so a missing marker cannot become a denial.
#[cfg(windows)]
fn output_child(child: Child, timeout: Duration) -> Result<OutputExchange, String> {
    const MAX_OUTPUT: usize = 4096;
    let mut child = ManagedChild {
        child,
        reaped: false,
    };
    drop(child.child.stdin.take());
    let mut stdout = child
        .child
        .stdout
        .take()
        .ok_or("denial child output pipe unavailable")?;
    let (sender, receiver) = mpsc::sync_channel(1);
    let io_thread = thread::Builder::new()
        .name("sandbox-denial-output".to_owned())
        .spawn(move || {
            let result = (|| {
                let mut bytes = Vec::new();
                Read::by_ref(&mut stdout)
                    .take(MAX_OUTPUT as u64 + 1)
                    .read_to_end(&mut bytes)
                    .map_err(|error| format!("denial child output failed: {error}"))?;
                if bytes.len() > MAX_OUTPUT {
                    return Err("denial child output exceeded bound".to_owned());
                }
                String::from_utf8(bytes).map_err(|_| "denial child output was not UTF-8".to_owned())
            })();
            let _ = sender.send(result);
        })
        .map_err(|error| format!("denial output thread failed: {error}"))?;
    let deadline = Instant::now() + timeout;
    let received = receiver.recv_timeout(timeout);
    let status = match &received {
        Ok(_) => match child.until_exit(deadline) {
            Ok(status) => status,
            Err(error) => {
                child.terminate()?;
                join_io(io_thread)?;
                return Err(format!("denial child exit failed: {error}"));
            }
        },
        Err(_) => {
            child.terminate()?;
            join_io(io_thread)?;
            let partial = receiver
                .try_recv()
                .ok()
                .and_then(Result::ok)
                .unwrap_or_default();
            return Err(format!(
                "denial child output timed out or disconnected; child reaped; partial={}",
                partial.trim()
            ));
        }
    };
    join_io(io_thread)?;
    let output = received.map_err(|_| "denial child output unavailable")??;
    Ok(OutputExchange { output, status })
}

pub fn process_smoke() -> Result<(), String> {
    let html = include_str!("../../../fixtures/welcome.html").to_owned();
    let expected = Document::parse(&html)
        .and_then(|document| document.layout(960))
        .map_err(|error| format!("local probe fixture failed: {error}"))?;
    let frame = encode(&Message::Render { html, width: 960 })
        .map_err(|error| format!("local probe encode failed: {error}"))?;
    let healthy = exchange(child_command()?, frame.clone(), EXCHANGE_TIMEOUT)?;
    if !healthy.status.success() || healthy.reply != Message::Display(expected.clone()) {
        return Err("healthy child did not return the actual expected document layout".to_owned());
    }
    let mut malformed = frame.clone();
    *malformed
        .first_mut()
        .ok_or("encoded request unexpectedly empty")? = b'X';
    let rejected = exchange(child_command()?, malformed, EXCHANGE_TIMEOUT)?;
    if rejected.status.success()
        || !matches!(
            rejected.reply,
            Message::Error {
                code: ErrorCode::Protocol,
                ..
            }
        )
    {
        return Err("malformed child request did not fail closed".to_owned());
    }
    let restarted = exchange(child_command()?, frame, EXCHANGE_TIMEOUT)?;
    if !restarted.status.success() || restarted.reply != Message::Display(expected) {
        return Err("new child after protocol failure did not recover".to_owned());
    }
    println!("PROCESS_SMOKE_OK healthy=1 malformed_rejected=1 restarted=1 spawned=3 reaped=3");
    println!("Local lifecycle probe only; no sandbox, site isolation or authenticated peer claim.");
    Ok(())
}

/// Opt-in real AppContainer launch/render probe. The ordinary trusted application
/// path is unchanged; no remote input or fail-open alternative is introduced.
#[cfg(windows)]
pub fn sandbox_smoke() -> Result<(), String> {
    use std::sync::Arc;
    let source = std::env::current_exe().map_err(|error| error.to_string())?;
    let sandbox = Arc::new(
        crate::windows_sandbox::RendererSandbox::prepare(&source)
            .map_err(|error| format!("sandbox preparation failed: {error}"))?,
    );
    let result = sandbox_render_probe(&sandbox).and_then(|()| sandbox_denial_probe(&sandbox));
    let cleanup = Arc::try_unwrap(sandbox)
        .map_err(|_| "sandbox still owned by child".to_owned())
        .and_then(|sandbox| {
            sandbox
                .cleanup()
                .map_err(|error| format!("sandbox profile/staging cleanup failed: {error}"))
        });
    match (result, cleanup) {
        (Ok(()), Ok(())) => {
            println!("SANDBOX_CLEANUP profile_deleted=1 staging_removed=1");
            println!("SANDBOX_SMOKE_OK token=verified render=equal denials=verified cleanup=ok");
            println!(
                "Opt-in development probe; origin/IPC/TLS and production sandbox gates remain unmet."
            );
            Ok(())
        }
        (Err(error), Ok(())) => {
            println!("SANDBOX_CLEANUP profile_deleted=1 staging_removed=1");
            Err(error)
        }
        (Ok(()), Err(error)) => Err(error.to_owned()),
        (Err(error), Err(cleanup)) => Err(format!("{error}; {cleanup}")),
    }
}

#[cfg(windows)]
fn sandbox_render_probe(
    sandbox: &std::sync::Arc<crate::windows_sandbox::RendererSandbox>,
) -> Result<(), String> {
    let html = include_str!("../../../fixtures/welcome.html").to_owned();
    let expected = Document::parse(&html)
        .and_then(|document| document.layout(960))
        .map_err(|error| format!("sandbox fixture failed: {error}"))?;
    let frame = encode(&Message::Render { html, width: 960 }).map_err(|error| error.to_string())?;
    let command = || {
        let mut command = Command::new(sandbox.executable());
        command.arg("--renderer-worker");
        command.current_dir(
            sandbox
                .executable()
                .parent()
                .unwrap_or(std::path::Path::new(".")),
        );
        command
    };
    let ordinary = exchange(command(), frame.clone(), EXCHANGE_TIMEOUT)?;
    if !ordinary.status.success() || ordinary.reply != Message::Display(expected.clone()) {
        return Err("ordinary staged renderer control failed".to_owned());
    }
    let child = crate::windows_process::spawn_sandboxed(&command(), sandbox)
        .map_err(|error| format!("sandbox launch failed: {error}"))?;
    let policy = child
        .verified_token()
        .ok_or("sandbox launch lacks measured token policy")?;
    let rendered = exchange_child(child, frame, EXCHANGE_TIMEOUT)?;
    if !rendered.status.success() || rendered.reply != Message::Display(expected) {
        return Err("sandbox renderer did not return the actual expected display".to_owned());
    }
    println!(
        "SANDBOX_RENDER app={} sid_equal=1 integrity_rid={} capabilities={} equal=1 spawned=2 reaped=2",
        u8::from(policy.appcontainer),
        policy.integrity_rid,
        policy.capability_count
    );
    Ok(())
}

#[cfg(windows)]
fn sandbox_denial_probe(
    sandbox: &std::sync::Arc<crate::windows_sandbox::RendererSandbox>,
) -> Result<(), String> {
    use std::net::{Ipv4Addr, TcpListener};

    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))
        .map_err(|error| format!("loopback listener failed: {error}"))?;
    listener
        .set_nonblocking(true)
        .map_err(|error| format!("loopback listener setup failed: {error}"))?;
    let port = listener
        .local_addr()
        .map_err(|error| format!("loopback listener address failed: {error}"))?
        .port();
    let fixture_before = std::fs::read(sandbox.fixture())
        .map_err(|error| format!("deny fixture setup failed: {error}"))?;
    let command = || {
        let mut command = Command::new(sandbox.executable());
        command
            .arg("--sandbox-denial-worker")
            .arg(sandbox.fixture())
            .arg(port.to_string());
        command.current_dir(
            sandbox
                .executable()
                .parent()
                .unwrap_or(std::path::Path::new(".")),
        );
        command
    };

    let ordinary_child = crate::windows_process::spawn(&command())
        .map_err(|error| format!("ordinary denial control launch failed: {error}"))?;
    let ordinary = output_child(ordinary_child, EXCHANGE_TIMEOUT)?;
    if !ordinary.status.success()
        || ordinary.output.lines().last() != Some("SANDBOX_PROBE app=0 read=0 write=0 network=0")
    {
        return Err(format!(
            "ordinary denial positive control failed: exit={:?} marker=invalid",
            ordinary.status.code()
        ));
    }
    if std::fs::read(sandbox.fixture()).map_err(|error| error.to_string())? != fixture_before {
        return Err("ordinary control changed the deny fixture".to_owned());
    }

    // The ordinary helper really connected and sent one fixed synthetic byte.
    let accept_deadline = Instant::now() + Duration::from_secs(1);
    let (mut stream, peer) = loop {
        match listener.accept() {
            Ok(connection) => break connection,
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                if Instant::now() >= accept_deadline {
                    return Err("ordinary loopback positive control was not accepted".to_owned());
                }
                thread::sleep(Duration::from_millis(2));
            }
            Err(error) => return Err(format!("ordinary loopback accept failed: {error}")),
        }
    };
    if peer.ip() != Ipv4Addr::LOCALHOST {
        return Err("loopback control peer was not IPv4 loopback".to_owned());
    }
    stream
        .set_read_timeout(Some(Duration::from_secs(1)))
        .map_err(|error| format!("loopback control timeout failed: {error}"))?;
    let mut synthetic = [0u8; 1];
    stream
        .read_exact(&mut synthetic)
        .map_err(|error| format!("loopback control read failed: {error}"))?;
    if synthetic != *b"S" {
        return Err("loopback control marker mismatch".to_owned());
    }
    let mut trailing = [0u8; 1];
    if stream
        .read(&mut trailing)
        .map_err(|error| format!("loopback control EOF failed: {error}"))?
        != 0
    {
        return Err("loopback control sent trailing data".to_owned());
    }
    drop(stream);
    println!(
        "SANDBOX_DENIAL_CONTROL app=0 read=0 write=0 network=0 accepted=1 fixture_unchanged=1"
    );

    let child = crate::windows_process::spawn_sandboxed(&command(), sandbox)
        .map_err(|error| format!("sandbox denial launch failed: {error}"))?;
    let policy = child
        .verified_token()
        .ok_or("sandbox denial child lacks measured token policy")?;
    let denied_result = output_child(child, EXCHANGE_TIMEOUT);
    if std::fs::read(sandbox.fixture()).map_err(|error| error.to_string())? != fixture_before {
        return Err("AppContainer changed the deny fixture".to_owned());
    }
    match listener.accept() {
        Ok((_stream, _peer)) => {
            return Err("AppContainer loopback connection was unexpectedly accepted".to_owned());
        }
        Err(error) if error.kind() == io::ErrorKind::WouldBlock => {}
        Err(error) => return Err(format!("sandbox loopback accept check failed: {error}")),
    }
    let denied = denied_result?;
    if policy.integrity_rid != 4096 || !policy.appcontainer || policy.capability_count != 0 {
        return Err("sandbox denial child token policy changed after launch".to_owned());
    }
    if !denied
        .output
        .lines()
        .any(|line| line == "SANDBOX_PROBE_FILES app=1 read=5 write=5")
    {
        return Err(format!(
            "AppContainer file denial marker missing: exit={:?} output={}",
            denied.status.code(),
            denied.output.trim()
        ));
    }
    // The file-denial and fixture observations are earned independently of the
    // network observation: the child may still fail its later loopback probe.
    println!("SANDBOX_DENIAL_OBSERVATION accepted=0 fixture_unchanged=1");
    println!("SANDBOX_PROBE_VERIFIED_FILES app=1 read=5 write=5");
    if !denied.status.success()
        || denied.output.lines().last() != Some("SANDBOX_PROBE app=1 read=5 write=5 network=10013")
    {
        return Err(format!(
            "AppContainer exact denial observation failed: exit={:?} output={}",
            denied.status.code(),
            denied.output.trim()
        ));
    }
    println!("SANDBOX_PROBE_VERIFIED app=1 read=5 write=5 network=10013");
    println!(
        "SANDBOX_DENIAL ordinary_app=0 sandbox_app={} sid_equal=1 integrity_rid={} capabilities={} read=5 write=5 network=10013 ordinary_accepted=1 accepted=0 fixture_unchanged=1",
        u8::from(policy.appcontainer),
        policy.integrity_rid,
        policy.capability_count
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn valid_wire_display_for_another_viewport_is_rejected_at_request_boundary() {
        let display = opensequoia_engine::DisplayList {
            width: 640,
            height: 32,
            items: Vec::new(),
        };
        // This is valid framed data, so protocol shape validation alone accepts it.
        let frame = encode(&Message::Display(display.clone())).unwrap();
        let reply = read_message(&mut Cursor::new(frame)).unwrap();
        assert!(
            validate_reply(reply.clone(), true, 320)
                .unwrap_err()
                .contains("viewport")
        );
        assert_eq!(validate_reply(reply.clone(), true, 640).unwrap(), display);
        assert!(validate_reply(reply, false, 640).is_err());
    }

    #[test]
    fn worker_uses_real_parse_and_layout() {
        let mut input = Cursor::new(
            encode(&Message::Render {
                html: "<p>Actual 🌲 content</p>".to_owned(),
                width: 640,
            })
            .unwrap(),
        );
        let mut output = Vec::new();
        serve(&mut input, &mut output).unwrap();
        let Message::Display(display) = read_message(&mut Cursor::new(output)).unwrap() else {
            panic!("worker did not return a display list");
        };
        assert_eq!(display.width, 640);
        assert!(display.items.iter().any(|item| match item {
            opensequoia_engine::DisplayItem::Text(run) => run.text.contains("🌲"),
            opensequoia_engine::DisplayItem::Fill(_) => false,
        }));
    }

    #[test]
    fn styled_text_at_the_ipc_font_bound_round_trips_through_worker() {
        let mut input = Cursor::new(
            encode(&Message::Render {
                html: "<p style=\"font-size:256px\">X</p>".to_owned(),
                width: 800,
            })
            .unwrap(),
        );
        let mut output = Vec::new();
        serve(&mut input, &mut output).unwrap();
        let Message::Display(display) = read_message(&mut Cursor::new(output)).unwrap() else {
            panic!("worker did not return a display list");
        };
        let run = match display.items.first() {
            Some(opensequoia_engine::DisplayItem::Text(run)) => run,
            Some(opensequoia_engine::DisplayItem::Fill(_)) => {
                panic!("styled text run was replaced by a fill")
            }
            None => panic!("styled text run missing"),
        };
        assert_eq!(run.font_size, 256);
    }

    #[test]
    fn worker_rejects_malformed_and_wrong_direction_frames() {
        for frame in [
            vec![0u8; 16],
            encode(&Message::Error {
                code: ErrorCode::Parse,
                message: "wrong direction".to_owned(),
            })
            .unwrap(),
        ] {
            let mut output = Vec::new();
            assert!(serve(&mut Cursor::new(frame), &mut output).is_err());
            assert!(matches!(
                read_message(&mut Cursor::new(output)).unwrap(),
                Message::Error {
                    code: ErrorCode::Protocol,
                    ..
                }
            ));
        }
    }

    #[test]
    fn unfinished_io_thread_returns_bounded_error_and_fixture_is_released() {
        let (release, blocked) = mpsc::sync_channel(1);
        let (finished, completion) = mpsc::sync_channel(1);
        let io_thread = thread::spawn(move || {
            let _ = blocked.recv();
            let _ = finished.send(());
        });
        let (returned, result_channel) = mpsc::sync_channel(1);
        let supervisor = thread::spawn(move || {
            let _ = returned.send(join_io(io_thread));
        });
        let started = Instant::now();
        let result = result_channel.recv_timeout(Duration::from_secs(3));
        let elapsed = started.elapsed();
        // Release the fixture even if a future unconditional join regresses and
        // the test deadline expires, so that regression fails without hanging
        // the test process. This is not a pipe cancellation test.
        release.send(()).unwrap();
        completion.recv_timeout(REAP_TIMEOUT).unwrap();
        supervisor.join().unwrap();
        assert!(
            result
                .expect("helper failed to return within test deadline")
                .unwrap_err()
                .contains("I/O did not stop")
        );
        // One-second helper deadline plus scheduling allowance. An unconditional
        // join would miss the test deadline and only return after fixture release.
        assert!(elapsed < Duration::from_secs(3));
    }

    #[test]
    #[cfg(windows)]
    fn supervisor_timeout_kills_and_reaps_actual_child() {
        let root = std::env::var_os("SystemRoot").unwrap();
        let executable =
            std::path::PathBuf::from(root).join("System32/WindowsPowerShell/v1.0/powershell.exe");
        let mut command = Command::new(executable);
        command.args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "Start-Sleep -Seconds 30",
        ]);
        let started = Instant::now();
        let error = exchange(command, vec![0; 16], Duration::from_millis(150))
            .err()
            .expect("controlled sleeping child must time out");
        assert!(error.contains("child reaped"), "{error}");
        // 150ms exchange timeout plus at most one-second reap and one-second I/O
        // completion allowance; 3s catches waiting for the helper's full30s sleep.
        assert!(started.elapsed() < Duration::from_secs(3));
    }
    #[test]
    fn block_background_fill_round_trips_through_worker() {
        let mut input = Cursor::new(
            encode(&Message::Render {
                html: "<body style=\"background-color:#102030\"><p>x</p></body>".to_owned(),
                width: 800,
            })
            .unwrap(),
        );
        let mut output = Vec::new();
        serve(&mut input, &mut output).unwrap();
        let Message::Display(display) = read_message(&mut Cursor::new(output)).unwrap() else {
            panic!("worker did not return a display list");
        };
        let Some(opensequoia_engine::DisplayItem::Fill(fill)) = display.items.first() else {
            panic!("background fill missing from worker output");
        };
        assert_eq!(fill.width, 768);
        assert!(display.items.iter().any(
            |item| matches!(item, opensequoia_engine::DisplayItem::Text(run) if run.text == "x")
        ));
    }
}
