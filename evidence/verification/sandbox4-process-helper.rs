//! Development-only real process probe, not part of the OpenSequoia package.
//! All fixtures are synthetic and all execution uses the production Win32 modules.
#[path = "../../crates/app/src/windows_process.rs"]
mod windows_process;
#[path = "../../crates/app/src/windows_sandbox.rs"]
mod windows_sandbox;

use std::{
    io::{self, BufRead, BufReader, Read, Write},
    os::windows::io::{AsRawHandle, FromRawHandle, IntoRawHandle, OwnedHandle},
    path::PathBuf,
    process::{Child, Command, ExitStatus, Stdio},
    sync::{Arc, mpsc},
    thread,
    time::{Duration, Instant},
};
use windows_process::ContainedChild;
use windows_sandbox::RendererSandbox;
use windows_sys::Win32::{
    Foundation::{HANDLE, INVALID_HANDLE_VALUE, WAIT_OBJECT_0, WAIT_TIMEOUT},
    System::Threading::{
        OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SYNCHRONIZE, WaitForSingleObject,
    },
};

const PROBE_TIMEOUT: Duration = Duration::from_secs(6);

#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetHandleInformation(handle: *mut std::ffi::c_void, flags: *mut u32) -> i32;
    fn ReadFile(
        handle: *mut std::ffi::c_void,
        bytes: *mut u8,
        size: u32,
        read: *mut u32,
        overlapped: *mut std::ffi::c_void,
    ) -> i32;
}

fn main() {
    if let Err(error) = run() {
        eprintln!("SANDBOX_HELPER_ERROR {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let args: Vec<_> = std::env::args_os().collect();
    match args.get(1).and_then(|value| value.to_str()) {
        Some("sleep") => {
            std::thread::sleep(Duration::from_secs(30));
            Ok(())
        }
        Some("parent") => ordinary_parent(),
        Some("handle") => handle_worker(args.get(2).ok_or("missing handle value")?),
        Some("descendant") => descendant_worker(),
        Some("sandbox-parent") => sandbox_parent(),
        Some("sandbox-lifetime") => sandbox_lifetime(),
        Some("sandbox-parent-death") => sandbox_parent_death(),
        _ => Err("unknown helper mode".to_owned()),
    }
}

fn owned(handle: HANDLE) -> io::Result<OwnedHandle> {
    if handle.is_null() || handle == INVALID_HANDLE_VALUE {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: only a fresh successful OpenProcess result is transferred once.
    Ok(unsafe { OwnedHandle::from_raw_handle(handle) })
}

fn observe_process(id: u32) -> Result<OwnedHandle, String> {
    // SAFETY: numeric PID came from our owned child's private pipe/handle. Open
    // minimal rights before any parent death to rule out later PID-reuse races.
    let handle = unsafe {
        OpenProcess(
            PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_SYNCHRONIZE,
            0,
            id,
        )
    };
    owned(handle).map_err(|error| format!("OpenProcess failed for owned child: {error}"))
}

fn wait_owned_process(process: &OwnedHandle, milliseconds: u32) -> Result<(), String> {
    // SAFETY: exact owned observation handle remains live for the bounded wait.
    match unsafe { WaitForSingleObject(process.as_raw_handle(), milliseconds) } {
        WAIT_OBJECT_0 => Ok(()),
        WAIT_TIMEOUT => Err("owned process wait timed out".to_owned()),
        other => Err(format!("owned process wait failed: {other}")),
    }
}

fn ordinary_parent() -> Result<(), String> {
    let mut command = Command::new(std::env::current_exe().map_err(|error| error.to_string())?);
    command.arg("sleep");
    let child = windows_process::spawn(&command).map_err(|error| error.to_string())?;
    println!("{}", child.id().map_err(|error| error.to_string())?);
    std::io::stdout()
        .flush()
        .map_err(|error| error.to_string())?;
    // Parent stays alive so the observer can open the exact child handle.
    let _ = std::io::stdin().read(&mut [0u8; 1]);
    // Intentionally skip Rust Drop: OS parent-death handle closure must kill child.
    std::process::exit(0);
}

fn handle_worker(raw: &std::ffi::OsStr) -> Result<(), String> {
    let raw: usize = raw
        .to_str()
        .ok_or("handle value is not Unicode")?
        .parse()
        .map_err(|_| "handle value is not a bounded integer")?;
    let mut flags = 0u32;
    // SAFETY: initialized flags output; the integer is never dereferenced as a
    // Rust pointer. Invalid handles return FALSE instead of a fatal ReadFile.
    if unsafe { GetHandleInformation(raw as *mut _, &mut flags) } == 0 {
        println!("NOT_INHERITED");
        return Ok(());
    }
    let mut bytes = [0u8; 32];
    let mut read = 0;
    // SAFETY: bounded initialized output and count. The integer is only an OS
    // handle validity probe; invalid handles are rejected without dereferencing.
    let success = unsafe {
        ReadFile(
            raw as *mut _,
            bytes.as_mut_ptr(),
            32,
            &mut read,
            std::ptr::null_mut(),
        )
    };
    let inherited = success != 0 && read == 32 && bytes == [0x53; 32];
    println!(
        "{}",
        if inherited {
            "INHERITED"
        } else {
            "NOT_INHERITED"
        }
    );
    Ok(())
}

fn descendant_worker() -> Result<(), String> {
    let executable = std::env::current_exe().map_err(|error| error.to_string())?;
    match Command::new(executable)
        .arg("sleep")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    {
        Ok(mut child) => {
            child
                .kill()
                .map_err(|error| format!("descendant cleanup kill failed: {error}"))?;
            child
                .wait()
                .map_err(|error| format!("descendant cleanup wait failed: {error}"))?;
            println!("STARTED_AND_REAPED");
        }
        Err(error) => {
            let code = error
                .raw_os_error()
                .ok_or("descendant error lacks OS code")?;
            println!("DENIED:{code}");
        }
    }
    Ok(())
}

fn staged_command(sandbox: &RendererSandbox, mode: &str) -> Result<Command, String> {
    let mut command = Command::new(sandbox.executable());
    command.arg(mode).current_dir(
        sandbox
            .executable()
            .parent()
            .ok_or("staged executable has no parent")?,
    );
    Ok(command)
}

fn wait_child(child: &mut ContainedChild, deadline: Instant) -> Result<ExitStatus, String> {
    loop {
        if let Some(status) = child.try_wait().map_err(|error| error.to_string())? {
            return Ok(status);
        }
        if Instant::now() >= deadline {
            return Err("child exit timed out".to_owned());
        }
        thread::sleep(Duration::from_millis(2));
    }
}

fn join_cancelable(reader: thread::JoinHandle<()>) -> Result<(), String> {
    let deadline = Instant::now() + Duration::from_secs(1);
    while !reader.is_finished() {
        if Instant::now() >= deadline {
            return Err("helper I/O thread did not stop; cleanup is unverified".to_owned());
        }
        windows_process::cancel_io(&reader).map_err(|error| error.to_string())?;
        thread::sleep(Duration::from_millis(2));
    }
    reader.join().map_err(|_| "helper I/O reader panicked")?;
    Ok(())
}

fn bounded_output(child: &mut ContainedChild) -> Result<(String, ExitStatus), String> {
    drop(child.stdin.take());
    let mut stdout = child.stdout.take().ok_or("child stdout missing")?;
    let (sender, receiver) = mpsc::sync_channel(1);
    let reader = thread::spawn(move || {
        let mut bytes = Vec::new();
        let result = Read::by_ref(&mut stdout)
            .take(1025)
            .read_to_end(&mut bytes)
            .map_err(|error| error.to_string())
            .and_then(|_| {
                if bytes.len() > 1024 {
                    Err("child output exceeded bound".to_owned())
                } else {
                    String::from_utf8(bytes).map_err(|_| "child output not UTF-8".to_owned())
                }
            });
        let _ = sender.send(result);
    });
    let received = receiver.recv_timeout(PROBE_TIMEOUT);
    let status_result = if received.is_err() {
        let kill_result = child.kill().map_err(|error| error.to_string());
        let wait_result = wait_child(child, Instant::now() + Duration::from_secs(1));
        kill_result.and(wait_result)
    } else {
        wait_child(child, Instant::now() + Duration::from_secs(1))
    };
    let io_result = join_cancelable(reader);
    let status = status_result?;
    io_result?;
    if received.is_err() {
        return Err("child output timed out; child was killed and reaped".to_owned());
    }
    let output = received
        .map_err(|_| "child output timed out".to_owned())?
        .map_err(|error| format!("child output failed: {error}"))?;
    Ok((output, status))
}

fn sandbox_parent() -> Result<(), String> {
    let source = std::env::current_exe().map_err(|error| error.to_string())?;
    let sandbox = Arc::new(RendererSandbox::prepare(&source).map_err(|error| error.to_string())?);
    let child = windows_process::spawn_sandboxed(&staged_command(&sandbox, "sleep")?, &sandbox)
        .map_err(|error| format!("sandbox-parent launch failed: {error}"))?;
    println!("ROOT {}", sandbox.root().display());
    println!("CHILD {}", child.id().map_err(|error| error.to_string())?);
    std::io::stdout()
        .flush()
        .map_err(|error| error.to_string())?;
    let _ = std::io::stdin().read(&mut [0u8; 1]);
    // Deliberately bypass Drop. The outer probe must observe Job death and then
    // use the private owner receipt to recover this exact profile/staging tree.
    std::process::exit(0);
}

/// Value: protects=AppContainer Job, quota, handle, cancellation and cleanup controls;
/// fails_when=a production launch attribute, handle list, Job limit or lifetime guard regresses;
/// why_new=ordinary process tests cannot observe token-restricted descendants or inherited handles;
/// seam=spawn_sandboxed_control varies only the real OS positive-control inputs;
/// no_claim=origin isolation, exploit resistance, other Windows versions or network denial.
fn sandbox_lifetime() -> Result<(), String> {
    let source = std::env::current_exe().map_err(|error| error.to_string())?;
    let sandbox = Arc::new(RendererSandbox::prepare(&source).map_err(|error| error.to_string())?);
    let result = (|| {
        let wrong_path = windows_process::spawn_sandboxed(&Command::new(&source), &sandbox);
        if wrong_path.err().map(|error| error.kind()) != Some(io::ErrorKind::InvalidInput) {
            return Err("unstaged executable did not fail closed".to_owned());
        }
        println!("SANDBOX_LIFETIME_STAGE wrong_path=denied");

        let job_child =
            windows_process::spawn_sandboxed(&staged_command(&sandbox, "sleep")?, &sandbox)
                .map_err(|error| format!("Job-close child launch failed: {error}"))?;
        if !job_child
            .is_in_owned_job()
            .map_err(|error| error.to_string())?
        {
            return Err("sandbox child is not in its owned Job".to_owned());
        }
        let observation = observe_process(job_child.id().map_err(|error| error.to_string())?)?;
        drop(job_child); // Last Job handle closes while the child is still live.
        wait_owned_process(&observation, 2_000)?;
        println!("SANDBOX_LIFETIME_STAGE job_close=1");

        for (limit, expected) in [(2, "STARTED_AND_REAPED"), (1, "DENIED:1816")] {
            let mut child = windows_process::spawn_sandboxed_control(
                &staged_command(&sandbox, "descendant")?,
                &sandbox,
                &[],
                limit,
            )
            .map_err(|error| format!("descendant control launch failed: {error}"))?;
            let (output, status) = bounded_output(&mut child)?;
            if !status.success() {
                return Err(format!(
                    "descendant limit {limit} exited unsuccessfully: {status}"
                ));
            }
            if output.trim() != expected {
                return Err(format!(
                    "descendant limit {limit} returned {:?}, expected {expected}",
                    output.trim()
                ));
            }
            println!("SANDBOX_LIFETIME_STAGE quota={limit} result={expected}");
        }

        for include in [true, false] {
            let (write, read) = windows_process::pipe(false).map_err(|error| error.to_string())?;
            // SAFETY: fresh parent write handle is transferred to File exactly once.
            let mut writer = unsafe { std::fs::File::from_raw_handle(write.into_raw_handle()) };
            writer
                .write_all(&[0x53; 32])
                .map_err(|error| error.to_string())?;
            drop(writer);
            let mut command = staged_command(&sandbox, "handle")?;
            command.arg((read.as_raw_handle() as usize).to_string());
            let extra = if include {
                &[read.as_raw_handle()][..]
            } else {
                &[][..]
            };
            let mut child = windows_process::spawn_sandboxed_control(&command, &sandbox, extra, 1)
                .map_err(|error| format!("handle control launch failed: {error}"))?;
            let (output, status) = bounded_output(&mut child)?;
            let expected = if include {
                if !status.success() || output.trim() != "INHERITED" {
                    return Err("included handle did not read the exact private marker".to_owned());
                }
                "INHERITED"
            } else {
                if status.code() != Some(0xc0000008u32 as i32) || !output.trim().is_empty() {
                    return Err(format!(
                        "excluded handle did not produce exact strict-handle STATUS_INVALID_HANDLE: {status}"
                    ));
                }
                "0xc0000008"
            };
            println!("SANDBOX_LIFETIME_STAGE handle_include={include} result={expected}");
        }

        let mut sleeping =
            windows_process::spawn_sandboxed(&staged_command(&sandbox, "sleep")?, &sandbox)
                .map_err(|error| format!("cancellation child launch failed: {error}"))?;
        let mut output = sleeping.stdout.take().ok_or("cancellation pipe missing")?;
        let (started, start) = mpsc::sync_channel(1);
        let (finished, completion) = mpsc::sync_channel(1);
        let reader = thread::spawn(move || {
            let _ = started.send(());
            let outcome = output
                .read(&mut [0u8; 1])
                .map(|_| None)
                .unwrap_or_else(|error| error.raw_os_error());
            let _ = finished.send(outcome);
        });
        start
            .recv_timeout(Duration::from_secs(1))
            .map_err(|_| "cancellation did not start")?;
        let deadline = Instant::now() + Duration::from_secs(2);
        while !reader.is_finished() && Instant::now() < deadline {
            windows_process::cancel_io(&reader).map_err(|error| error.to_string())?;
            thread::sleep(Duration::from_millis(2));
        }
        if completion
            .recv_timeout(Duration::from_secs(1))
            .map_err(|_| "pipe read was not cancelled")?
            != Some(995)
        {
            return Err("pipe cancellation was not exact ERROR_OPERATION_ABORTED 995".to_owned());
        }
        reader.join().map_err(|_| "cancellation reader panicked")?;
        sleeping.kill().map_err(|error| error.to_string())?;
        wait_child(&mut sleeping, Instant::now() + Duration::from_secs(1))?;
        println!("SANDBOX_LIFETIME_STAGE cancellation=995");
        println!(
            "SANDBOX_LIFETIME job_close=1 quota_control=started quota_denied=1816 include_control=1 unrelated_handle=0xc0000008 cancellation=995 wrong_path=denied"
        );
        Ok(())
    })();
    let cleanup = Arc::try_unwrap(sandbox)
        .map_err(|_| "lifetime sandbox still owned".to_owned())
        .and_then(|sandbox| sandbox.cleanup().map_err(|error| error.to_string()));
    match (result, cleanup) {
        (Ok(()), Ok(())) => {
            println!("SANDBOX_LIFETIME_OK reaped=1 cleanup=1");
            Ok(())
        }
        (Err(error), Ok(())) => Err(error),
        (Ok(()), Err(error)) => Err(format!("lifetime cleanup failed: {error}")),
        (Err(error), Err(cleanup)) => Err(format!("{error}; lifetime cleanup failed: {cleanup}")),
    }
}

fn kill_std_child(child: &mut Child) -> Result<(), String> {
    if child
        .try_wait()
        .map_err(|error| error.to_string())?
        .is_none()
    {
        child.kill().map_err(|error| error.to_string())?;
    }
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        if child
            .try_wait()
            .map_err(|error| error.to_string())?
            .is_some()
        {
            return Ok(());
        }
        if Instant::now() >= deadline {
            return Err("helper parent did not reap after kill".to_owned());
        }
        thread::sleep(Duration::from_millis(2));
    }
}

/// Value: protects=noninherited Job death for a real AppContainer child and owned crash cleanup;
/// fails_when=a parent survives handle closure without killing its child or recovery deletes the wrong path;
/// why_new=ordinary parent-death coverage has no AppContainer SID/profile/staging lifetime;
/// seam=an actual helper parent is killed without Rust Drop, with no mocked OS behavior;
/// no_claim=universal crash recovery, arbitrary-path deletion or production sandbox acceptance.
fn sandbox_parent_death() -> Result<(), String> {
    let executable = std::env::current_exe().map_err(|error| error.to_string())?;
    let mut parent = Command::new(executable)
        .arg("sandbox-parent")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| format!("sandbox parent launch failed: {error}"))?;
    let stdout = parent
        .stdout
        .take()
        .ok_or("sandbox parent stdout missing")?;
    let (sender, receiver) = mpsc::sync_channel(1);
    let reader = thread::spawn(move || {
        let mut lines = Vec::new();
        let mut input = BufReader::new(stdout).take(2049);
        let result = (|| {
            for _ in 0..2 {
                let mut line = String::new();
                input
                    .read_line(&mut line)
                    .map_err(|error| error.to_string())?;
                lines.push(line);
            }
            Ok::<_, String>(lines)
        })();
        let _ = sender.send(result);
    });
    let received = receiver.recv_timeout(PROBE_TIMEOUT);
    let early_kill = if received.is_err() {
        Some(kill_std_child(&mut parent))
    } else {
        None
    };
    let io_result = join_cancelable(reader);
    let mut root: Option<PathBuf> = None;
    let result = (|| {
        if let Some(kill_result) = early_kill {
            kill_result?;
        }
        io_result?;
        let lines = received
            .map_err(|_| "sandbox parent metadata timed out".to_owned())?
            .map_err(|error| format!("sandbox parent metadata failed: {error}"))?;
        let root_line = lines.first().ok_or("missing sandbox parent root")?.trim();
        let child_line = lines.get(1).ok_or("missing sandbox parent child")?.trim();
        let parsed_root = PathBuf::from(
            root_line
                .strip_prefix("ROOT ")
                .ok_or("invalid root marker")?,
        );
        root = Some(parsed_root);
        let id: u32 = child_line
            .strip_prefix("CHILD ")
            .ok_or("invalid child marker")?
            .parse()
            .map_err(|_| "invalid child PID")?;
        let observation = observe_process(id)?;
        kill_std_child(&mut parent)?;
        wait_owned_process(&observation, 2_000)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = kill_std_child(&mut parent);
    }
    let cleanup = root
        .as_deref()
        .ok_or_else(|| "no owned sandbox root available for recovery".to_owned())
        .and_then(|root| {
            RendererSandbox::cleanup_abandoned(root).map_err(|error| error.to_string())
        });
    match (result, cleanup) {
        (Ok(()), Ok(())) => {
            println!(
                "SANDBOX_PARENT_DEATH killed=1 child_reaped=1 profile_deleted=1 staging_removed=1"
            );
            Ok(())
        }
        (Err(error), Ok(())) => Err(error),
        (Ok(()), Err(error)) => Err(format!("parent-death cleanup failed: {error}")),
        (Err(error), Err(cleanup)) => {
            Err(format!("{error}; parent-death cleanup failed: {cleanup}"))
        }
    }
}
