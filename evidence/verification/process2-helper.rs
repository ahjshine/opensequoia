//! Development-only process probe, not part of the OpenSequoia package.
#[path = "../../crates/app/src/windows_process.rs"]
mod windows_process;

use std::{io::{Read, Write}, process::Command, time::Duration};

#[link(name = "kernel32")]
unsafe extern "system" {
    fn ReadFile(handle: *mut std::ffi::c_void, bytes: *mut u8, size: u32, read: *mut u32, overlapped: *mut std::ffi::c_void) -> i32;
}

fn main() {
    let args: Vec<_> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("sleep") => std::thread::sleep(Duration::from_secs(30)),
        Some("parent") => {
            let mut command = Command::new(std::env::current_exe().unwrap());
            command.arg("sleep");
            let child = windows_process::spawn(&command).unwrap();
            println!("{}", child.id().unwrap());
            std::io::stdout().flush().unwrap();
            // Parent stays alive so observer can open the exact child handle.
            let _ = std::io::stdin().read(&mut [0u8; 1]);
            // Intentionally skip Rust Drop: OS parent-death handle closure must kill child.
            std::process::exit(0);
        }
        Some("handle") => {
            let raw: usize = args.get(2).unwrap().parse().unwrap();
            let mut bytes = [0u8; 32];
            let mut read = 0;
            // SAFETY: bounded initialized output buffer and read count. The passed
            // integer is an OS handle validity probe; invalid handles are rejected
            // by Windows. No output bytes are logged; compare private marker only.
            let success = unsafe { ReadFile(raw as *mut _, bytes.as_mut_ptr(), 32, &mut read, std::ptr::null_mut()) };
            let inherited = success != 0 && read == 32 && bytes == [0x53; 32];
            println!("{}", if inherited { "INHERITED" } else { "NOT_INHERITED" });
        }
        _ => std::process::exit(2),
    }
}
