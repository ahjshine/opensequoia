//! Documented Win32 renderer process ownership plus opt-in AppContainer launch.
//! Ordinary spawn remains a development control, not an origin/privilege sandbox.
//! Rust 1.98.1's custom Command attributes are nightly,
//! so this boundary calls CreateProcessW with a bounded STARTUPINFOEX directly.
//!
//! Sources: Microsoft CreateProcessW, UpdateProcThreadAttribute (JOB_LIST and
//! HANDLE_LIST), Job Object limits, and CancelSynchronousIo documentation.
//! All owned handles are closed once; attribute backing values stay live through
//! CreateProcessW; the Job is never inheritable and applies before execution.

#![deny(unsafe_op_in_unsafe_fn)]

use std::{
    ffi::{OsStr, OsString},
    fs::File,
    io,
    mem::size_of,
    os::windows::{
        ffi::OsStrExt,
        io::{AsRawHandle, FromRawHandle, IntoRawHandle, OwnedHandle},
        process::ExitStatusExt,
    },
    path::Path,
    process::{Command, ExitStatus},
    ptr,
    sync::Arc,
    thread::JoinHandle,
};

use windows_sys::Win32::{
    Foundation::{
        ERROR_INSUFFICIENT_BUFFER, ERROR_NOT_FOUND, GENERIC_WRITE, HANDLE, HANDLE_FLAG_INHERIT,
        INVALID_HANDLE_VALUE, SetHandleInformation, WAIT_OBJECT_0, WAIT_TIMEOUT,
    },
    Security::{SECURITY_ATTRIBUTES, SECURITY_CAPABILITIES},
    Storage::FileSystem::{
        CreateFileW, FILE_ATTRIBUTE_NORMAL, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
    },
    System::{
        IO::CancelSynchronousIo,
        JobObjects::{
            CreateJobObjectW, IsProcessInJob, JOB_OBJECT_LIMIT_ACTIVE_PROCESS,
            JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE, JOB_OBJECT_LIMIT_PROCESS_MEMORY,
            JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JobObjectExtendedLimitInformation,
            SetInformationJobObject, TerminateJobObject,
        },
        Pipes::CreatePipe,
        Threading::{
            CREATE_NO_WINDOW, CREATE_SUSPENDED, CREATE_UNICODE_ENVIRONMENT, CreateProcessW,
            DeleteProcThreadAttributeList, EXTENDED_STARTUPINFO_PRESENT, GetExitCodeProcess,
            GetProcessId, InitializeProcThreadAttributeList, LPPROC_THREAD_ATTRIBUTE_LIST,
            PROC_THREAD_ATTRIBUTE_HANDLE_LIST, PROC_THREAD_ATTRIBUTE_JOB_LIST,
            PROC_THREAD_ATTRIBUTE_SECURITY_CAPABILITIES, PROCESS_INFORMATION, ResumeThread,
            STARTF_USESTDHANDLES, STARTUPINFOEXW, UpdateProcThreadAttribute, WaitForSingleObject,
        },
    },
};

const MAX_ATTRIBUTE_BYTES: usize = 65_536;
const MAX_PROCESS_MEMORY: usize = 256 * 1024 * 1024;
const MAX_NATIVE_TEXT: usize = 32_767;

pub struct ContainedChild {
    process: OwnedHandle,
    // Last owned Job handle; never inherited or cloned. Closing kills members.
    job: OwnedHandle,
    // Declared after Job: the Job closes before a final sandbox owner can clean
    // up its profile/staging. Arc retains SID/profile authority throughout life.
    _sandbox: Option<Arc<crate::windows_sandbox::RendererSandbox>>,
    token_policy: Option<crate::windows_sandbox::TokenPolicy>,
    pub stdin: Option<File>,
    pub stdout: Option<File>,
}

impl ContainedChild {
    pub fn verified_token(&self) -> Option<crate::windows_sandbox::TokenPolicy> {
        self.token_policy
    }
    pub fn id(&self) -> io::Result<u32> {
        // SAFETY: valid owned process handle is queried, never a caller's PID.
        let id = unsafe { GetProcessId(self.process.as_raw_handle()) };
        if id == 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(id)
        }
    }
    pub fn try_wait(&mut self) -> io::Result<Option<ExitStatus>> {
        // SAFETY: process is a live owned process handle; no pointer is borrowed.
        let status = unsafe { WaitForSingleObject(self.process.as_raw_handle(), 0) };
        match status {
            WAIT_TIMEOUT => Ok(None),
            WAIT_OBJECT_0 => {
                let mut code = 0;
                // SAFETY: initialized writable exit code and valid process handle.
                if unsafe { GetExitCodeProcess(self.process.as_raw_handle(), &mut code) } == 0 {
                    return Err(io::Error::last_os_error());
                }
                Ok(Some(ExitStatus::from_raw(code)))
            }
            _ => Err(io::Error::last_os_error()),
        }
    }

    pub fn kill(&mut self) -> io::Result<()> {
        // SAFETY: unnamed Job is exclusively owned by this launch, not a global
        // Job or caller-provided handle. This kills all members, not an arbitrary PID.
        if unsafe { TerminateJobObject(self.job.as_raw_handle(), 1) } == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }

    pub fn is_in_owned_job(&self) -> io::Result<bool> {
        let mut result = 0;
        // SAFETY: both handles remain owned throughout the query; result initialized.
        if unsafe {
            IsProcessInJob(
                self.process.as_raw_handle(),
                self.job.as_raw_handle(),
                &mut result,
            )
        } == 0
        {
            return Err(io::Error::last_os_error());
        }
        Ok(result != 0)
    }
}

/// Cancel pending synchronous I/O on exactly this owned Rust thread. A not-found
/// result means no operation was pending; caller repeats while waiting boundedly.
pub fn cancel_io(thread: &JoinHandle<()>) -> io::Result<()> {
    // SAFETY: JoinHandle owns the native thread handle and remains borrowed, so
    // the caller cannot concurrently consume/join/close that same handle here.
    if unsafe { CancelSynchronousIo(thread.as_raw_handle()) } != 0 {
        return Ok(());
    }
    let error = io::Error::last_os_error();
    if error.raw_os_error() == Some(ERROR_NOT_FOUND as i32) {
        Ok(())
    } else {
        Err(error)
    }
}

fn owned(handle: HANDLE) -> io::Result<OwnedHandle> {
    if handle.is_null() || handle == INVALID_HANDLE_VALUE {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: only successful fresh Win32 handle-returning APIs call this helper;
    // ownership is transferred once and no other owner is retained.
    Ok(unsafe { OwnedHandle::from_raw_handle(handle) })
}

fn abort_launch(job: &OwnedHandle, process: &OwnedHandle, original: io::Error) -> io::Error {
    // SAFETY: exclusively owned fresh Job/process handles from this launch; no
    // caller-selected PID. Failed policy validation never resumes renderer code.
    let killed = unsafe { TerminateJobObject(job.as_raw_handle(), 1) } != 0;
    // SAFETY: owned process remains live while bounded termination is observed.
    let waited = unsafe { WaitForSingleObject(process.as_raw_handle(), 1000) };
    if killed && waited == WAIT_OBJECT_0 {
        original
    } else {
        io::Error::other(format!(
            "{original}; failed-launch cleanup unverified killed={killed} wait={waited}"
        ))
    }
}

pub(crate) fn pipe(parent_reads: bool) -> io::Result<(OwnedHandle, OwnedHandle)> {
    let attributes = SECURITY_ATTRIBUTES {
        nLength: size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: ptr::null_mut(),
        bInheritHandle: 1,
    };
    let (mut read, mut write) = (ptr::null_mut(), ptr::null_mut());
    // SAFETY: initialized output slots and sized SECURITY_ATTRIBUTES live for call.
    if unsafe { CreatePipe(&mut read, &mut write, &attributes, 0) } == 0 {
        return Err(io::Error::last_os_error());
    }
    let (read, write) = (owned(read)?, owned(write)?);
    let (parent, child) = if parent_reads {
        (read, write)
    } else {
        (write, read)
    };
    // SAFETY: parent is a live owned handle; clearing inheritance changes only it.
    if unsafe { SetHandleInformation(parent.as_raw_handle(), HANDLE_FLAG_INHERIT, 0) } == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok((parent, child))
}

struct AttributeList {
    backing: Vec<u128>,
    initialized: bool,
}

impl AttributeList {
    fn pointer(&mut self) -> LPPROC_THREAD_ATTRIBUTE_LIST {
        self.backing.as_mut_ptr().cast()
    }

    fn new(count: u32) -> io::Result<Self> {
        let mut bytes = 0;
        // SAFETY: documented sizing query uses null list and initialized size slot.
        let first =
            unsafe { InitializeProcThreadAttributeList(ptr::null_mut(), count, 0, &mut bytes) };
        let error = io::Error::last_os_error();
        if first != 0
            || error.raw_os_error() != Some(ERROR_INSUFFICIENT_BUFFER as i32)
            || bytes == 0
            || bytes > MAX_ATTRIBUTE_BYTES
        {
            return Err(io::Error::other("invalid startup attribute sizing result"));
        }
        // 16-byte alignment meets Win32 heap alignment; u128 has no invalid bit
        // patterns and is never accessed while Windows fills its opaque storage.
        let mut result = Self {
            backing: vec![0; bytes.div_ceil(size_of::<u128>())],
            initialized: false,
        };
        // SAFETY: backing is aligned, initialized and at least queried byte size.
        if unsafe { InitializeProcThreadAttributeList(result.pointer(), count, 0, &mut bytes) } == 0
        {
            let error = io::Error::last_os_error();
            return Err(error);
        }
        result.initialized = true;
        Ok(result)
    }

    fn set_handles(&mut self, attribute: u32, values: &mut [HANDLE]) -> io::Result<()> {
        // SAFETY: attribute is one of the documented HANDLE_LIST/JOB_LIST values,
        // values are live HANDLE arrays with exact checked length; spawn retains
        // both arrays and all handle owners through CreateProcessW.
        if unsafe {
            UpdateProcThreadAttribute(
                self.pointer(),
                0,
                attribute as usize,
                values.as_mut_ptr().cast(),
                std::mem::size_of_val(values),
                ptr::null_mut(),
                ptr::null(),
            )
        } == 0
        {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }

    fn set_security(&mut self, security: &mut SECURITY_CAPABILITIES) -> io::Result<()> {
        // SAFETY: exact documented structure size. Caller declares both SID
        // owner and SECURITY_CAPABILITIES before this list, retaining their
        // stable backing through CreateProcess AND attribute-list destruction.
        if unsafe {
            UpdateProcThreadAttribute(
                self.pointer(),
                0,
                PROC_THREAD_ATTRIBUTE_SECURITY_CAPABILITIES as usize,
                (security as *mut SECURITY_CAPABILITIES).cast(),
                size_of::<SECURITY_CAPABILITIES>(),
                ptr::null_mut(),
                ptr::null(),
            )
        } == 0
        {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }
}

impl Drop for AttributeList {
    fn drop(&mut self) {
        // SAFETY: successful new initializes exactly this still-owned backing.
        if self.initialized {
            unsafe { DeleteProcThreadAttributeList(self.pointer()) };
        }
    }
}

fn wide(value: &OsStr) -> io::Result<Vec<u16>> {
    let mut text: Vec<_> = value.encode_wide().take(MAX_NATIVE_TEXT).collect();
    if text.len() == MAX_NATIVE_TEXT {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "process text too long",
        ));
    }
    if text.contains(&0) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "NUL in process argument",
        ));
    }
    text.push(0);
    Ok(text)
}

fn append_argument(output: &mut Vec<u16>, argument: &OsStr) -> io::Result<()> {
    let text = wide(argument)?;
    output.push(b'"' as u16);
    let mut slashes = 0;
    for character in &text[..text.len() - 1] {
        if *character == b'\\' as u16 {
            slashes += 1;
            continue;
        }
        if *character == b'"' as u16 {
            output.extend(std::iter::repeat_n(b'\\' as u16, slashes * 2 + 1));
        } else {
            output.extend(std::iter::repeat_n(b'\\' as u16, slashes));
        }
        output.push(*character);
        slashes = 0;
    }
    output.extend(std::iter::repeat_n(b'\\' as u16, slashes * 2));
    output.push(b'"' as u16);
    Ok(())
}

/// Launch fixed trusted program/argument data, never a shell command line. The
/// caller must choose its program; this helper performs no PATH lookup and copies
/// only OS loader environment variables rather than caller environment/secrets.
pub fn spawn(command: &Command) -> io::Result<ContainedChild> {
    spawn_with_handles(command, &[])
}

/// Opt-in trusted renderer/probe only. Any failure aborts; no unrestricted
/// fallback. The supplied program must be exactly the owned staged executable.
pub fn spawn_sandboxed(
    command: &Command,
    sandbox: &Arc<crate::windows_sandbox::RendererSandbox>,
) -> io::Result<ContainedChild> {
    spawn_sandboxed_control(command, sandbox, &[], 1)
}

/// Development-only positive controls use the identical launch path with an
/// explicit inheritable sentinel or a two-process Job. Production remains one
/// process with exactly three inherited stdio handles; no unrestricted fallback.
pub fn spawn_sandboxed_control(
    command: &Command,
    sandbox: &Arc<crate::windows_sandbox::RendererSandbox>,
    extra: &[HANDLE],
    active_limit: u32,
) -> io::Result<ContainedChild> {
    if Path::new(command.get_program()) != sandbox.executable() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "sandbox program differs from staged executable",
        ));
    }
    spawn_config(command, extra, active_limit, Some(sandbox))
}

fn spawn_with_handles(command: &Command, extra: &[HANDLE]) -> io::Result<ContainedChild> {
    spawn_with_policy(command, extra, 1)
}

// Private policy parameter permits a same-workload test control with two members.
// Production spawn/spawn_with_handles always choose one and expose no caller knob.
fn spawn_with_policy(
    command: &Command,
    extra: &[HANDLE],
    active_limit: u32,
) -> io::Result<ContainedChild> {
    spawn_config(command, extra, active_limit, None)
}

fn spawn_config(
    command: &Command,
    extra: &[HANDLE],
    active_limit: u32,
    sandbox: Option<&Arc<crate::windows_sandbox::RendererSandbox>>,
) -> io::Result<ContainedChild> {
    if !(1..=2).contains(&active_limit) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "unsupported process limit",
        ));
    }
    let executable = Path::new(command.get_program());
    if !executable.is_absolute() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "process path must be absolute",
        ));
    }
    let application = wide(executable.as_os_str())?;
    let mut arguments = Vec::new();
    append_argument(&mut arguments, executable.as_os_str())?;
    for argument in command.get_args() {
        arguments.push(b' ' as u16);
        append_argument(&mut arguments, argument)?;
        if arguments.len() >= MAX_NATIVE_TEXT {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "process command line too long",
            ));
        }
    }
    if arguments.len() >= 32_767 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "process command line too long",
        ));
    }
    arguments.push(0);
    let current_directory = command
        .get_current_dir()
        .unwrap_or_else(|| executable.parent().unwrap_or(Path::new(".")));
    let directory = wide(current_directory.as_os_str())?;
    let mut environment_values = Vec::new();
    for name in ["SystemRoot", "WINDIR"] {
        if let Some(value) = std::env::var_os(name) {
            let mut entry = OsString::from(name);
            entry.push("=");
            entry.push(value);
            environment_values.push(entry);
        }
    }
    if let Some(sandbox) = sandbox {
        // Observed fix for ERROR_ENVVAR_NOT_FOUND (203): this host's AppContainer
        // setup requires LOCALAPPDATA. Supply only the fresh SID-RX stage, never
        // the user's real profile; APPDATA/USERPROFILE were not required here.
        let mut entry = OsString::from("LOCALAPPDATA=");
        entry.push(
            sandbox
                .executable()
                .parent()
                .ok_or_else(|| io::Error::other("sandbox executable has no staging directory"))?,
        );
        environment_values.push(entry);
    }
    environment_values.sort_by_key(|entry| entry.to_string_lossy().to_lowercase());
    let mut environment = Vec::new();
    for entry in environment_values {
        environment.extend(wide(&entry)?);
    }
    environment.push(0);
    if environment.len() == 1 {
        environment.push(0)
    }

    // SAFETY: null security/name gives a fresh non-inheritable unnamed Job.
    let job = owned(unsafe { CreateJobObjectW(ptr::null(), ptr::null()) })?;
    let limits = JOBOBJECT_EXTENDED_LIMIT_INFORMATION {
        BasicLimitInformation:
            windows_sys::Win32::System::JobObjects::JOBOBJECT_BASIC_LIMIT_INFORMATION {
                LimitFlags: JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE
                    | JOB_OBJECT_LIMIT_ACTIVE_PROCESS
                    | JOB_OBJECT_LIMIT_PROCESS_MEMORY,
                ActiveProcessLimit: active_limit,
                ..Default::default()
            },
        ProcessMemoryLimit: MAX_PROCESS_MEMORY,
        ..Default::default()
    };
    // SAFETY: live Job and initialized correctly sized limits. No application
    // memory remains referenced by Windows after SetInformationJobObject returns.
    if unsafe {
        SetInformationJobObject(
            job.as_raw_handle(),
            JobObjectExtendedLimitInformation,
            (&limits as *const JOBOBJECT_EXTENDED_LIMIT_INFORMATION).cast(),
            size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
        )
    } == 0
    {
        return Err(io::Error::last_os_error());
    }
    let (stdin, child_in) = pipe(false)?;
    let (stdout, child_out) = pipe(true)?;
    let inherit = SECURITY_ATTRIBUTES {
        nLength: size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: ptr::null_mut(),
        bInheritHandle: 1,
    };
    let nul: Vec<u16> = "NUL\0".encode_utf16().collect();
    // SAFETY: trusted OS NUL device, valid terminated path and security structure.
    let child_error = owned(unsafe {
        CreateFileW(
            nul.as_ptr(),
            GENERIC_WRITE,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            &inherit,
            OPEN_EXISTING,
            FILE_ATTRIBUTE_NORMAL,
            ptr::null_mut(),
        )
    })?;
    let mut handles = vec![
        child_in.as_raw_handle(),
        child_out.as_raw_handle(),
        child_error.as_raw_handle(),
    ];
    handles.extend_from_slice(extra);
    // Both value arrays own stable heap backing and are declared before the
    // attribute list, so they outlive DeleteProcThreadAttributeList as required.
    let mut jobs = vec![job.as_raw_handle()];
    let mut security = sandbox.map(|sandbox| sandbox.capabilities());
    let mut attributes = AttributeList::new(if sandbox.is_some() { 3 } else { 2 })?;
    attributes.set_handles(PROC_THREAD_ATTRIBUTE_HANDLE_LIST, &mut handles)?;
    attributes.set_handles(PROC_THREAD_ATTRIBUTE_JOB_LIST, &mut jobs)?;
    if let Some(security) = security.as_mut() {
        attributes.set_security(security)?;
    }
    let startup = STARTUPINFOEXW {
        StartupInfo: windows_sys::Win32::System::Threading::STARTUPINFOW {
            cb: size_of::<STARTUPINFOEXW>() as u32,
            dwFlags: STARTF_USESTDHANDLES,
            hStdInput: child_in.as_raw_handle(),
            hStdOutput: child_out.as_raw_handle(),
            hStdError: child_error.as_raw_handle(),
            ..Default::default()
        },
        lpAttributeList: attributes.pointer(),
    };
    let mut process = PROCESS_INFORMATION::default();
    // SAFETY: all terminated buffers, initialized structures, attribute lists,
    // arrays and owned handles remain live through this call. Explicit HANDLE_LIST
    // admits only the three stdio ends (extra handles are private test control).
    // JOB_LIST assigns the configured Job as part of creation before execution.
    if unsafe {
        CreateProcessW(
            application.as_ptr(),
            arguments.as_mut_ptr(),
            ptr::null(),
            ptr::null(),
            1,
            CREATE_NO_WINDOW
                | CREATE_UNICODE_ENVIRONMENT
                | EXTENDED_STARTUPINFO_PRESENT
                | if sandbox.is_some() {
                    CREATE_SUSPENDED
                } else {
                    0
                },
            environment.as_ptr().cast(),
            directory.as_ptr(),
            &startup.StartupInfo,
            &mut process,
        )
    } == 0
    {
        return Err(io::Error::other(format!(
            "CreateProcessW failed: {}",
            io::Error::last_os_error()
        )));
    }
    let process_handle = owned(process.hProcess)?;
    let initial_thread = owned(process.hThread)?;
    let token_policy = match sandbox
        .map(|sandbox| {
            crate::windows_sandbox::verify_process(process_handle.as_raw_handle(), sandbox)
        })
        .transpose()
    {
        Ok(policy) => policy,
        Err(error) => return Err(abort_launch(&job, &process_handle, error)),
    };
    // SAFETY: parent pipe handles are fresh owned file-like handles; transfer to
    // std File once. They were explicitly marked non-inheritable above.
    let stdin = unsafe { File::from_raw_handle(stdin.into_raw_handle()) };
    let stdout = unsafe { File::from_raw_handle(stdout.into_raw_handle()) };
    let child = ContainedChild {
        process: process_handle,
        job,
        _sandbox: sandbox.cloned(),
        token_policy,
        stdin: Some(stdin),
        stdout: Some(stdout),
    };
    let identity = child.id().and_then(|id| {
        child
            .is_in_owned_job()
            .map(|in_job| id == process.dwProcessId && in_job)
    });
    if !matches!(identity, Ok(true)) {
        let error = identity
            .err()
            .unwrap_or_else(|| io::Error::other("child process/Job identity mismatch"));
        return Err(abort_launch(&child.job, &child.process, error));
    }
    if sandbox.is_some() {
        // SAFETY: exact owned initial thread created suspended; token/SID/IL/
        // capabilities and Job verified before any renderer code executes.
        let previous = unsafe { ResumeThread(initial_thread.as_raw_handle()) };
        if previous == u32::MAX {
            return Err(abort_launch(
                &child.job,
                &child.process,
                io::Error::last_os_error(),
            ));
        }
        if previous != 1 {
            return Err(abort_launch(
                &child.job,
                &child.process,
                io::Error::other("unexpected sandbox suspend count"),
            ));
        }
    }
    Ok(child)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        io::{BufRead, BufReader, Read, Write},
        path::PathBuf,
        process::Stdio,
        sync::{OnceLock, mpsc},
        thread,
        time::{Duration, Instant},
    };
    use windows_sys::Win32::System::Threading::{
        OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SYNCHRONIZE,
    };

    fn helper() -> &'static Path {
        static HELPER: OnceLock<PathBuf> = OnceLock::new();
        HELPER
            .get_or_init(|| {
                let workspace = Path::new(env!("CARGO_MANIFEST_DIR"))
                    .parent()
                    .unwrap()
                    .parent()
                    .unwrap();
                let executable = std::env::current_exe().unwrap();
                let dependencies = executable.parent().unwrap();
                let rlib = std::fs::read_dir(dependencies)
                    .unwrap()
                    .filter_map(Result::ok)
                    .filter(|entry| {
                        entry
                            .file_name()
                            .to_string_lossy()
                            .starts_with("libwindows_sys-")
                            && entry
                                .path()
                                .extension()
                                .is_some_and(|extension| extension == "rlib")
                    })
                    .max_by_key(|entry| entry.metadata().unwrap().modified().unwrap())
                    .unwrap()
                    .path();
                let output = workspace.join("target/sandbox4/process-helper.exe");
                std::fs::create_dir_all(output.parent().unwrap()).unwrap();
                let result = Command::new("rustc")
                    .arg("--edition=2024")
                    .arg(workspace.join("evidence/verification/sandbox4-process-helper.rs"))
                    .arg("--extern")
                    .arg(format!("windows_sys={}", rlib.display()))
                    .arg("-L")
                    .arg(format!("dependency={}", dependencies.display()))
                    .arg("-o")
                    .arg(&output)
                    .output()
                    .unwrap();
                std::fs::write(
                    workspace.join("target/sandbox4/helper-build.log"),
                    &result.stderr,
                )
                .unwrap();
                assert!(
                    result.status.success(),
                    "{}",
                    String::from_utf8_lossy(&result.stderr)
                );
                output
            })
            .as_path()
    }

    fn bounded_output(child: &mut ContainedChild) -> String {
        let stdout = child.stdout.take().unwrap();
        let (sender, result) = mpsc::sync_channel(1);
        let reader = thread::spawn(move || {
            let mut output = String::new();
            let read = stdout.take(512).read_to_string(&mut output);
            let _ = sender.send((read, output));
        });
        let received = result.recv_timeout(Duration::from_secs(5));
        if received.is_err() {
            child.kill().unwrap();
            cancel_io(&reader).unwrap();
        }
        let (read, output) = received.expect("process probe must finish within deadline");
        read.unwrap();
        reader.join().unwrap();
        output
    }

    fn powershell(script: &str) -> Command {
        let root = std::env::var_os("SystemRoot").unwrap();
        let mut command =
            Command::new(Path::new(&root).join("System32/WindowsPowerShell/v1.0/powershell.exe"));
        command.args(["-NoProfile", "-NonInteractive", "-Command", script]);
        command
    }

    #[test]
    fn job_membership_is_established_at_spawn_and_job_close_kills_child() {
        let mut child = spawn(&powershell("Start-Sleep -Seconds 30")).unwrap();
        assert!(child.is_in_owned_job().unwrap());
        assert!(child.try_wait().unwrap().is_none());
        let process = child.process;
        drop(child.job);
        // SAFETY: valid owned process handle remains open for observed termination.
        assert_eq!(
            unsafe { WaitForSingleObject(process.as_raw_handle(), 2_000) },
            WAIT_OBJECT_0
        );
    }

    #[test]
    fn job_active_process_limit_rejects_descendant_creation() {
        let script = "$ErrorActionPreference='Stop'; try { $s=New-Object Diagnostics.ProcessStartInfo; $s.FileName=$PSHOME+'\\powershell.exe'; $s.Arguments='-NoProfile -NonInteractive -Command Start-Sleep -Seconds 30'; $s.UseShellExecute=$false; $p=[Diagnostics.Process]::Start($s); $p.Kill(); if(-not $p.WaitForExit(2000)) { 'CLEANUP_FAILED'; exit 1 }; 'STARTED_AND_REAPED'; exit 0 } catch { $e=$_.Exception; while($e.InnerException) { $e=$e.InnerException }; if($e -is [ComponentModel.Win32Exception]) { 'DENIED:'+ $e.NativeErrorCode; exit 0 }; 'UNEXPECTED_ERROR'; exit 1 }";
        for (active_limit, expected) in [(2, "STARTED_AND_REAPED"), (1, "DENIED:1816")] {
            let mut child = spawn_with_policy(&powershell(script), &[], active_limit).unwrap();
            let output = bounded_output(&mut child);
            let deadline = Instant::now() + Duration::from_secs(5);
            while child.try_wait().unwrap().is_none() && Instant::now() < deadline {
                thread::sleep(Duration::from_millis(5))
            }
            assert!(child.try_wait().unwrap().unwrap().success(), "{output}");
            assert_eq!(output.trim(), expected);
        }
    }

    #[test]
    fn real_synchronous_pipe_read_can_be_cancelled() {
        let (read, write) = pipe(true).unwrap();
        // This pipe helper normally returns parent read/child write; writer stays
        // alive so EOF cannot make the test pass in place of cancellation.
        // SAFETY: transfer a fresh owned read-only pipe handle exactly once.
        let mut file = unsafe { File::from_raw_handle(read.into_raw_handle()) };
        let (started, start) = mpsc::sync_channel(1);
        let (done, result) = mpsc::sync_channel(1);
        let thread = thread::spawn(move || {
            started.send(()).unwrap();
            let error = file.read(&mut [0u8; 1]).unwrap_err();
            done.send(error.raw_os_error()).unwrap();
        });
        start.recv_timeout(Duration::from_secs(1)).unwrap();
        let deadline = Instant::now() + Duration::from_secs(2);
        while !thread.is_finished() && Instant::now() < deadline {
            cancel_io(&thread).unwrap();
            thread::sleep(Duration::from_millis(5));
        }
        drop(write); // Guaranteed fixture release even if cancellation regresses.
        assert_eq!(
            result.recv_timeout(Duration::from_secs(1)).unwrap(),
            Some(995)
        );
        thread.join().unwrap();
    }

    #[test]
    fn native_argument_quoting_preserves_quotes_and_trailing_slashes() {
        for (input, expected) in [
            ("hello", "\"hello\""),
            ("a b", "\"a b\""),
            ("a\"b", "\"a\\\"b\""),
            ("a\\b\\", "\"a\\b\\\\\""),
        ] {
            let mut output = Vec::new();
            append_argument(&mut output, OsStr::new(input)).unwrap();
            assert_eq!(String::from_utf16(&output).unwrap(), expected);
        }
        assert!(wide(OsStr::new("a\0b")).is_err());
        assert!(wide(OsStr::new(&"x".repeat(MAX_NATIVE_TEXT))).is_err());
    }

    #[test]
    fn unrelated_inheritable_handle_is_excluded_with_positive_control() {
        for include_sentinel in [true, false] {
            let (write, read) = pipe(false).unwrap();
            // SAFETY: fresh owned parent write pipe is transferred to File once.
            let mut writer = unsafe { File::from_raw_handle(write.into_raw_handle()) };
            writer.write_all(&[0x53; 32]).unwrap();
            drop(writer);
            let mut command = Command::new(helper());
            command.args(["handle", &(read.as_raw_handle() as usize).to_string()]);
            let mut child = if include_sentinel {
                spawn_with_handles(&command, &[read.as_raw_handle()]).unwrap()
            } else {
                spawn(&command).unwrap()
            };
            let output = bounded_output(&mut child);
            assert_eq!(
                output.trim(),
                if include_sentinel {
                    "INHERITED"
                } else {
                    "NOT_INHERITED"
                }
            );
        }
    }

    #[test]
    fn parent_death_closes_noninherited_job_and_kills_actual_worker() {
        let mut parent = Command::new(helper())
            .arg("parent")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let stdout = parent.stdout.take().unwrap();
        let (sender, receiver) = mpsc::sync_channel(1);
        let reader = thread::spawn(move || {
            let mut line = String::new();
            let result = BufReader::new(stdout).take(32).read_line(&mut line);
            let _ = sender.send((result, line));
        });
        let result = receiver.recv_timeout(Duration::from_secs(5));
        if result.is_err() {
            let _ = parent.kill();
            cancel_io(&reader).unwrap();
        }
        let (read, line) = result.expect("parent probe must publish private child ID");
        read.unwrap();
        reader.join().unwrap();
        let id = line.trim().parse::<u32>().unwrap();
        // SAFETY: numeric PID came only from our owned helper's private pipe; open
        // minimal observation rights before ending parent to avoid PID reuse races.
        let child = owned(unsafe {
            OpenProcess(
                PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_SYNCHRONIZE,
                0,
                id,
            )
        })
        .unwrap();
        parent.kill().unwrap(); // Intentionally bypass helper's Rust destructors.
        let deadline = Instant::now() + Duration::from_secs(2);
        while parent.try_wait().unwrap().is_none() && Instant::now() < deadline {
            thread::sleep(Duration::from_millis(5))
        }
        assert!(parent.try_wait().unwrap().is_some());
        // SAFETY: exact owned child observation handle stays open after parent exit.
        assert_eq!(
            unsafe { WaitForSingleObject(child.as_raw_handle(), 2_000) },
            WAIT_OBJECT_0
        );
    }
}
