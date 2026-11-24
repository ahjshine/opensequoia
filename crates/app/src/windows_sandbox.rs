//! Opt-in development AppContainer preparation and token validation.
//! Documented Windows OS services only; no browser implementation is delegated.
//! Source/API contract: docs/discovery/appcontainer.md. This is not full site,
//! origin, capability-IPC, download/extension or production sandbox acceptance.

#![deny(unsafe_op_in_unsafe_fn)]

use std::{
    fs::{self, OpenOptions},
    io::{self, Read, Write},
    mem::size_of,
    os::windows::{
        fs::OpenOptionsExt,
        io::{AsRawHandle, FromRawHandle, OwnedHandle},
    },
    path::{Path, PathBuf},
    ptr,
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use windows_sys::Win32::{
    Foundation::{ERROR_INSUFFICIENT_BUFFER, HANDLE, LocalFree},
    Security::{
        Authorization::{
            EXPLICIT_ACCESS_W, GRANT_ACCESS, SE_FILE_OBJECT, SetEntriesInAclW, SetSecurityInfo,
            TRUSTEE_IS_SID, TRUSTEE_IS_UNKNOWN, TRUSTEE_W,
        },
        CreateWellKnownSid, DACL_SECURITY_INFORMATION, EqualSid, FreeSid, GetLengthSid,
        GetTokenInformation, IsValidSid,
        Isolation::{CreateAppContainerProfile, DeleteAppContainerProfile},
        PROTECTED_DACL_SECURITY_INFORMATION, PSID, SECURITY_CAPABILITIES,
        TOKEN_APPCONTAINER_INFORMATION, TOKEN_INFORMATION_CLASS, TOKEN_MANDATORY_LABEL,
        TOKEN_QUERY, TOKEN_USER, TokenAppContainerSid, TokenCapabilities, TokenIntegrityLevel,
        TokenIsAppContainer, TokenUser, WinLocalSystemSid,
    },
    Storage::FileSystem::{
        FILE_ALL_ACCESS, FILE_ATTRIBUTE_REPARSE_POINT, FILE_FLAG_BACKUP_SEMANTICS,
        FILE_FLAG_OPEN_REPARSE_POINT, FILE_GENERIC_EXECUTE, FILE_GENERIC_READ, READ_CONTROL,
        WRITE_DAC,
    },
    System::Threading::{GetCurrentProcess, OpenProcessToken},
};

const MAX_TOKEN_BYTES: usize = 65_536;
const MAX_SID_BYTES: usize = 68;
const MARKER: &[u8] = b"OpenSequoia generated sandbox fixture\n";
const OWNER_MARKER: &[u8] = b"OpenSequoia owned AppContainer profile\n";
const MAX_OWNER_BYTES: usize = 256;

// Exact documented Winsock ABI from SDK WinSock2.h and windows-sys 0.61.2.
// Kept local because the root integration intentionally enables no WinSock feature.
#[repr(C)]
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
struct WsaData {
    version: u16,
    high_version: u16,
    max_sockets: u16,
    max_udp_datagram: u16,
    vendor_info: *mut i8,
    description: [i8; 257],
    system_status: [i8; 129],
}

#[repr(C)]
#[cfg(target_arch = "x86")]
struct WsaData {
    version: u16,
    high_version: u16,
    description: [i8; 257],
    system_status: [i8; 129],
    max_sockets: u16,
    max_udp_datagram: u16,
    vendor_info: *mut i8,
}

#[repr(C)]
struct SockAddrIn {
    family: u16,
    port: u16,
    address: u32,
    zero: [i8; 8],
}

#[repr(C)]
struct WsaPollFd {
    socket: usize,
    events: i16,
    returned_events: i16,
}

const _: () = assert!(size_of::<SockAddrIn>() == 16);
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
const _: () = assert!(size_of::<WsaData>() == 408);
#[cfg(target_arch = "x86")]
const _: () = assert!(size_of::<WsaData>() == 400);

#[link(name = "ws2_32")]
unsafe extern "system" {
    #[link_name = "WSAStartup"]
    fn wsa_startup(version: u16, data: *mut WsaData) -> i32;
    #[link_name = "WSACleanup"]
    fn wsa_cleanup() -> i32;
    #[link_name = "WSAGetLastError"]
    fn wsa_get_last_error() -> i32;
    #[link_name = "socket"]
    fn socket(family: i32, socket_type: i32, protocol: i32) -> usize;
    #[link_name = "connect"]
    fn connect(socket: usize, address: *const SockAddrIn, bytes: i32) -> i32;
    #[link_name = "send"]
    fn send(socket: usize, bytes: *const i8, length: i32, flags: i32) -> i32;
    #[link_name = "ioctlsocket"]
    fn ioctl_socket(socket: usize, command: i32, value: *mut u32) -> i32;
    #[link_name = "WSAPoll"]
    fn wsa_poll(fds: *mut WsaPollFd, count: u32, timeout_ms: i32) -> i32;
    #[link_name = "getsockopt"]
    fn get_socket_option(
        socket: usize,
        level: i32,
        option: i32,
        value: *mut i8,
        bytes: *mut i32,
    ) -> i32;
    #[link_name = "closesocket"]
    fn closesocket(socket: usize) -> i32;
}

struct WsaSession;
impl WsaSession {
    fn start() -> io::Result<Self> {
        // SAFETY: all-zero is valid for this C integer/pointer/array output
        // structure; WSAStartup writes at most its exact SDK-defined layout.
        let mut data: WsaData = unsafe { std::mem::zeroed() };
        // SAFETY: live correctly aligned WSADATA output and requested version 2.2.
        let result = unsafe { wsa_startup(0x0202, &mut data) };
        if result != 0 {
            return Err(io::Error::other(format!("WSAStartup failed WSA={result}")));
        }
        Ok(Self)
    }
}
impl Drop for WsaSession {
    fn drop(&mut self) {
        // SAFETY: this guard exists only after one successful WSAStartup; paired
        // exactly once after the socket guard declared later has closed.
        let _ = unsafe { wsa_cleanup() };
    }
}

struct Socket(usize);
impl Drop for Socket {
    fn drop(&mut self) {
        // SAFETY: this is one fresh non-INVALID_SOCKET Winsock descriptor.
        let _ = unsafe { closesocket(self.0) };
    }
}

fn winsock_loopback(port: u16) -> io::Result<i32> {
    let _session = WsaSession::start()?;
    // SAFETY: fixed AF_INET/SOCK_STREAM/IPPROTO_TCP values, no borrowed memory.
    let raw = unsafe { socket(2, 1, 6) };
    if raw == usize::MAX {
        // SAFETY: immediately reads this thread's Winsock error after failure.
        let error = unsafe { wsa_get_last_error() };
        return Err(io::Error::other(format!(
            "Winsock socket failed WSA={error}"
        )));
    }
    let socket = Socket(raw);
    let address = SockAddrIn {
        family: 2,
        port: port.to_be(),
        // Native-endian integer bytes are exactly 127.0.0.1 in memory.
        address: u32::from_ne_bytes([127, 0, 0, 1]),
        zero: [0; 8],
    };
    let mut nonblocking = 1u32;
    // SAFETY: live socket and writable u32 for documented FIONBIO input.
    if unsafe { ioctl_socket(socket.0, 0x8004_667e_u32 as i32, &mut nonblocking) } != 0 {
        // SAFETY: immediately reads this thread's error after ioctl failure.
        let error = unsafe { wsa_get_last_error() };
        return Err(io::Error::other(format!(
            "Winsock nonblocking setup failed WSA={error}"
        )));
    }
    // SAFETY: live socket and exact 16-byte initialized SOCKADDR_IN.
    if unsafe { connect(socket.0, &address, size_of::<SockAddrIn>() as i32) } != 0 {
        // SAFETY: immediately reads this thread's error after connect failure.
        let error = unsafe { wsa_get_last_error() };
        if error == 10013 {
            return Ok(error);
        }
        if error != 10035 && error != 10036 {
            return Err(io::Error::other(format!(
                "Winsock loopback connect failed WSA={error}"
            )));
        }
        let mut poll = WsaPollFd {
            socket: socket.0,
            events: 0x0010, // POLLOUT
            returned_events: 0,
        };
        // SAFETY: one live writable WSAPOLLFD and bounded two-second timeout.
        let ready = unsafe { wsa_poll(&mut poll, 1, 2_000) };
        if ready < 0 {
            // SAFETY: immediately reads this thread's error after WSAPoll failure.
            let error = unsafe { wsa_get_last_error() };
            return Err(io::Error::other(format!("Winsock poll failed WSA={error}")));
        }
        let mut socket_error = 0i32;
        let mut error_bytes = size_of::<i32>() as i32;
        // SAFETY: SOL_SOCKET/SO_ERROR writes one initialized i32 and length.
        if unsafe {
            get_socket_option(
                socket.0,
                0xffff,
                0x1007,
                (&mut socket_error as *mut i32).cast(),
                &mut error_bytes,
            )
        } != 0
        {
            // SAFETY: immediately reads this thread's error after getsockopt.
            let error = unsafe { wsa_get_last_error() };
            return Err(io::Error::other(format!(
                "Winsock SO_ERROR query failed WSA={error}"
            )));
        }
        if socket_error == 10013 {
            return Ok(socket_error);
        }
        if socket_error != 0 {
            return Err(io::Error::other(format!(
                "Winsock asynchronous connect failed WSA={socket_error}"
            )));
        }
        if ready == 0 {
            return Err(io::Error::other(
                "Winsock WSAPoll loopback connect timed out with SO_ERROR=0",
            ));
        }
    }
    // SAFETY: live connected socket and one initialized synthetic byte; flags 0.
    if unsafe { send(socket.0, b"S".as_ptr().cast(), 1, 0) } != 1 {
        // SAFETY: immediately reads this thread's error after send failure.
        let error = unsafe { wsa_get_last_error() };
        return Err(io::Error::other(format!(
            "Winsock loopback send failed WSA={error}"
        )));
    }
    Ok(0)
}

// Initialized u32 backing meets SID alignment and makes SID ownership Send/Sync
// without an unsafe trait implementation. Windows APIs below only read this SID.
struct Sid {
    words: Vec<u32>,
    bytes: usize,
}

impl Sid {
    fn pointer(&self) -> PSID {
        self.words.as_ptr().cast_mut().cast()
    }

    fn data(&self) -> &[u8] {
        // SAFETY: the initialized u32 backing is live for at least bytes and is
        // never mutated while shared; u8 may observe every integer bit pattern.
        unsafe { std::slice::from_raw_parts(self.words.as_ptr().cast(), self.bytes) }
    }

    fn local_system() -> io::Result<Self> {
        let mut words = vec![0u32; MAX_SID_BYTES.div_ceil(size_of::<u32>())];
        let mut bytes = MAX_SID_BYTES as u32;
        // SAFETY: initialized aligned SID output has the advertised writable
        // extent; LocalSystem needs no domain SID. No borrowed pointer escapes.
        if unsafe {
            CreateWellKnownSid(
                WinLocalSystemSid,
                ptr::null_mut(),
                words.as_mut_ptr().cast(),
                &mut bytes,
            )
        } == 0
        {
            return Err(io::Error::last_os_error());
        }
        let sid = Self {
            words,
            bytes: bytes as usize,
        };
        // SAFETY: successful bounded OS SID output remains live and immutable.
        if sid.bytes < 8
            || sid.bytes > MAX_SID_BYTES
            || unsafe { IsValidSid(sid.pointer()) } == 0
            || unsafe { GetLengthSid(sid.pointer()) } as usize != sid.bytes
        {
            return Err(io::Error::other("invalid LocalSystem SID"));
        }
        Ok(sid)
    }

    fn low_integrity_rid(&self) -> Option<u32> {
        let data = self.data();
        if data.len() != 12 || data[0] != 1 || data[1] != 1 || data[2..8] != [0, 0, 0, 0, 0, 16] {
            return None;
        }
        Some(u32::from_le_bytes(data[8..12].try_into().ok()?))
    }

    fn copy_validated(bytes: &[u8]) -> io::Result<Self> {
        if bytes.len() < 8
            || bytes.len() > MAX_SID_BYTES
            || bytes[1] > 15
            || bytes.len() != 8 + 4 * usize::from(bytes[1])
        {
            return Err(io::Error::other("invalid SID size"));
        }
        let mut words = vec![0u32; bytes.len().div_ceil(size_of::<u32>())];
        // SAFETY: initialized u32 allocation covers the checked byte length;
        // copy writes bytes without creating invalid u32 bit patterns/aliasing.
        unsafe { ptr::copy_nonoverlapping(bytes.as_ptr(), words.as_mut_ptr().cast(), bytes.len()) };
        let sid = Self {
            words,
            bytes: bytes.len(),
        };
        // SAFETY: SID header/subauthority count and backing extent checked above.
        if unsafe { IsValidSid(sid.pointer()) } == 0 {
            return Err(io::Error::other("invalid SID"));
        }
        Ok(sid)
    }
}

struct OsSid(PSID);
impl Drop for OsSid {
    fn drop(&mut self) {
        // SAFETY: only successful CreateAppContainerProfile SID outputs enter this
        // owner, FreeSid is the documented paired allocator, exactly once.
        if !self.0.is_null() {
            unsafe { FreeSid(self.0) };
        }
    }
}

struct FreshProfile {
    name: Vec<u16>,
    owned: bool,
}
impl Drop for FreshProfile {
    fn drop(&mut self) {
        // SAFETY: only an exact S_OK fresh creation sets owned; never delete a
        // collision/derived existing profile. Explicit normal cleanup is below.
        if self.owned {
            unsafe { DeleteAppContainerProfile(self.name.as_ptr()) };
        }
    }
}

struct TokenBuffer {
    words: Vec<usize>,
    bytes: usize,
}
impl TokenBuffer {
    fn query(token: HANDLE, class: TOKEN_INFORMATION_CLASS) -> io::Result<Self> {
        let mut bytes = 0;
        // SAFETY: live borrowed token; null/zero is documented size query.
        let first = unsafe { GetTokenInformation(token, class, ptr::null_mut(), 0, &mut bytes) };
        if first != 0
            || io::Error::last_os_error().raw_os_error() != Some(ERROR_INSUFFICIENT_BUFFER as i32)
            || bytes == 0
            || bytes as usize > MAX_TOKEN_BYTES
        {
            return Err(io::Error::other("invalid token size query"));
        }
        let mut result = Self {
            words: vec![0; (bytes as usize).div_ceil(size_of::<usize>())],
            bytes: bytes as usize,
        };
        let requested = bytes;
        // SAFETY: initialized aligned allocation is at least requested bytes;
        // token information types used here contain integers/pointers only.
        if unsafe {
            GetTokenInformation(
                token,
                class,
                result.words.as_mut_ptr().cast(),
                requested,
                &mut bytes,
            )
        } == 0
        {
            return Err(io::Error::last_os_error());
        }
        if bytes == 0 || bytes > requested {
            return Err(io::Error::other("invalid token reply length"));
        }
        result.bytes = bytes as usize;
        Ok(result)
    }

    fn data(&self) -> &[u8] {
        // SAFETY: initialized integer backing is live, at least bytes long, and
        // not concurrently modified. u8 may observe any integer representation.
        unsafe { std::slice::from_raw_parts(self.words.as_ptr().cast(), self.bytes) }
    }

    fn sid(&self, pointer: PSID) -> io::Result<Sid> {
        let start = self.words.as_ptr() as usize;
        let offset = (pointer as usize)
            .checked_sub(start)
            .ok_or_else(|| io::Error::other("foreign token SID"))?;
        let header = self
            .data()
            .get(
                offset
                    ..offset
                        .checked_add(8)
                        .ok_or_else(|| io::Error::other("SID overflow"))?,
            )
            .ok_or_else(|| io::Error::other("truncated token SID"))?;
        let bytes = 8 + 4 * usize::from(header[1]);
        let end = offset
            .checked_add(bytes)
            .ok_or_else(|| io::Error::other("SID overflow"))?;
        Sid::copy_validated(
            self.data()
                .get(offset..end)
                .ok_or_else(|| io::Error::other("truncated token SID"))?,
        )
    }
}

fn process_token(process: HANDLE) -> io::Result<OwnedHandle> {
    let mut token = ptr::null_mut();
    // SAFETY: borrowed live process handle and initialized output slot. QUERY
    // only, no token duplication/impersonation/adjustment authority requested.
    if unsafe { OpenProcessToken(process, TOKEN_QUERY, &mut token) } == 0 {
        return Err(io::Error::last_os_error());
    }
    if token.is_null() {
        return Err(io::Error::other("null process token"));
    }
    // SAFETY: fresh successful handle transferred to one owner.
    Ok(unsafe { OwnedHandle::from_raw_handle(token) })
}

fn token_user() -> io::Result<Sid> {
    // SAFETY: documented current-process pseudo handle is queried, not closed.
    let token = process_token(unsafe { GetCurrentProcess() })?;
    let buffer = TokenBuffer::query(token.as_raw_handle(), TokenUser)?;
    if buffer.bytes < size_of::<TOKEN_USER>() {
        return Err(io::Error::other("truncated token user"));
    }
    // SAFETY: size checked; C TOKEN_USER contains pointer/integer bit patterns,
    // read_unaligned avoids assuming API-returned layout pointer alignment.
    let user = unsafe { ptr::read_unaligned(buffer.data().as_ptr().cast::<TOKEN_USER>()) };
    buffer.sid(user.User.Sid)
}

fn is_appcontainer(token: HANDLE) -> io::Result<bool> {
    let buffer = TokenBuffer::query(token, TokenIsAppContainer)?;
    let value = u32::from_le_bytes(
        buffer
            .data()
            .try_into()
            .map_err(|_| io::Error::other("invalid AppContainer flag"))?,
    );
    match value {
        0 => Ok(false),
        1 => Ok(true),
        _ => Err(io::Error::other("invalid AppContainer flag")),
    }
}

struct LocalAllocation(*mut std::ffi::c_void);
impl Drop for LocalAllocation {
    fn drop(&mut self) {
        // SAFETY: nonnull output from SetEntriesInAclW, paired LocalFree once.
        if !self.0.is_null() {
            unsafe { LocalFree(self.0) };
        }
    }
}

fn set_owned_acl(path: &Path, user: &Sid, app: Option<&Sid>) -> io::Result<()> {
    let file = OpenOptions::new()
        .access_mode(READ_CONTROL | WRITE_DAC)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT | FILE_FLAG_BACKUP_SEMANTICS)
        .open(path)?;
    use std::os::windows::fs::MetadataExt;
    if file.metadata()?.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
        return Err(io::Error::other("reparse point in sandbox staging"));
    }
    let entry = |sid: &Sid, access| EXPLICIT_ACCESS_W {
        grfAccessPermissions: access,
        grfAccessMode: GRANT_ACCESS,
        grfInheritance: 0,
        Trustee: TRUSTEE_W {
            TrusteeForm: TRUSTEE_IS_SID,
            TrusteeType: TRUSTEE_IS_UNKNOWN,
            ptstrName: sid.pointer().cast(),
            ..Default::default()
        },
    };
    let system = Sid::local_system()?;
    let mut entries = vec![
        entry(user, FILE_ALL_ACCESS),
        entry(&system, FILE_ALL_ACCESS),
    ];
    if let Some(app) = app {
        entries.push(entry(app, FILE_GENERIC_READ | FILE_GENERIC_EXECUTE));
    }
    let mut acl = ptr::null_mut();
    // SAFETY: initialized entries contain live validated SID pointers; null old
    // ACL intentionally creates user/SYSTEM plus optional exact AppContainer RX.
    let status = unsafe {
        SetEntriesInAclW(
            entries.len() as u32,
            entries.as_ptr(),
            ptr::null(),
            &mut acl,
        )
    };
    if status != 0 {
        return Err(io::Error::from_raw_os_error(status as i32));
    }
    let allocation = LocalAllocation(acl.cast());
    if allocation.0.is_null() {
        return Err(io::Error::other("null staging ACL"));
    }
    // SAFETY: live opened non-reparse object, initialized ACL and no owner/group
    // change. Only freshly created owned staging objects call this helper.
    let status = unsafe {
        SetSecurityInfo(
            file.as_raw_handle(),
            SE_FILE_OBJECT,
            DACL_SECURITY_INFORMATION | PROTECTED_DACL_SECURITY_INFORMATION,
            ptr::null_mut(),
            ptr::null_mut(),
            acl,
            ptr::null(),
        )
    };
    if status != 0 {
        return Err(io::Error::from_raw_os_error(status as i32));
    }
    Ok(())
}

/// A fresh profile and explicit, private staging directory. No borrowed native
/// pointer/handle is stored, so cross-thread ownership needs no unsafe traits.
pub struct RendererSandbox {
    name: Vec<u16>,
    sid: Sid,
    root: PathBuf,
    canonical_root: Option<PathBuf>,
    stage: PathBuf,
    controls: PathBuf,
    owner: PathBuf,
    binary: PathBuf,
    fixture: PathBuf,
    profile_created: bool,
    directory_created: bool,
}

impl RendererSandbox {
    pub fn prepare(source: &Path) -> io::Result<Self> {
        if !source.is_absolute() {
            return Err(io::Error::other("sandbox source must be absolute"));
        }
        let source = source.canonicalize()?;
        let mut input = OpenOptions::new()
            .read(true)
            .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT)
            .open(&source)?;
        use std::os::windows::fs::MetadataExt;
        let metadata = input.metadata()?;
        if !metadata.is_file()
            || metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
            || metadata.len() > 64 * 1024 * 1024
        {
            return Err(io::Error::other("invalid trusted sandbox executable"));
        }
        static SEQUENCE: AtomicU64 = AtomicU64::new(1);
        let nanos = u64::try_from(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_err(io::Error::other)?
                .as_nanos(),
        )
        .map_err(io::Error::other)?;
        let name = format!(
            "OpenSequoia.Probe.{:x}.{:x}.{:x}",
            std::process::id(),
            nanos,
            SEQUENCE.fetch_add(1, Ordering::Relaxed)
        );
        if name.len() > 64 {
            return Err(io::Error::other("sandbox name exceeds OS bound"));
        }
        let name_wide: Vec<u16> = name.encode_utf16().chain(Some(0)).collect();
        let sandbox4 = source
            .ancestors()
            .find(|path| {
                path.file_name() == Some(std::ffi::OsStr::new("sandbox4"))
                    && path.parent().and_then(Path::file_name)
                        == Some(std::ffi::OsStr::new("target"))
            })
            .ok_or_else(|| {
                io::Error::other("sandbox probe executable must be under target/sandbox4")
            })?;
        let base = sandbox4.join("runs");
        fs::create_dir_all(&base)?;
        reject_reparse(&base)?;
        let canonical_base = base.canonicalize()?;
        if canonical_base.parent() != Some(sandbox4.canonicalize()?.as_path()) {
            return Err(io::Error::other("sandbox runs directory escaped sandbox4"));
        }
        let base = local_drive_path(&canonical_base)?;
        let mut os_sid = ptr::null_mut();
        // SAFETY: fixed generated ASCII bounded name, live terminated strings,
        // null/zero capabilities, initialized SID output. Never reuse collisions.
        let result = unsafe {
            CreateAppContainerProfile(
                name_wide.as_ptr(),
                name_wide.as_ptr(),
                name_wide.as_ptr(),
                ptr::null(),
                0,
                &mut os_sid,
            )
        };
        if result != 0 {
            return Err(io::Error::other(format!(
                "AppContainer profile creation failed HRESULT={result:#x}"
            )));
        }
        let mut profile = FreshProfile {
            name: name_wide.clone(),
            owned: true,
        };
        let owned_sid = OsSid(os_sid);
        if owned_sid.0.is_null() {
            return Err(io::Error::other("null AppContainer SID"));
        }
        // SAFETY: successful API guarantees allocated SID; IsValidSid verifies
        // structure before GetLengthSid or bounded copy; allocator stays owned.
        if unsafe { IsValidSid(owned_sid.0) } == 0 {
            return Err(io::Error::other("invalid AppContainer SID"));
        }
        let bytes = unsafe { GetLengthSid(owned_sid.0) } as usize;
        if !(8..=MAX_SID_BYTES).contains(&bytes) {
            return Err(io::Error::other("invalid AppContainer SID extent"));
        }
        // SAFETY: successful owned validated SID has at least GetLengthSid bytes.
        let sid =
            Sid::copy_validated(unsafe { std::slice::from_raw_parts(owned_sid.0.cast(), bytes) })?;
        let root = base.join(&name);
        let mut sandbox = Self {
            name: name_wide,
            sid,
            stage: root.join("stage"),
            controls: root.join("controls"),
            owner: root.join("owner.txt"),
            binary: root.join("stage/renderer.exe"),
            fixture: root.join("controls/deny.dat"),
            root,
            canonical_root: None,
            profile_created: true,
            directory_created: false,
        };
        profile.owned = false; // sandbox now owns this exact fresh profile
        fs::create_dir(&sandbox.root)?; // existing directory is never owned/deleted
        sandbox.directory_created = true;
        // Canonical parent resolved before creation; generated ASCII child name
        // has no traversal/symlink segment. Compare again before cleanup.
        sandbox.canonical_root = Some(sandbox.root.canonicalize()?);
        let user = token_user()?;
        set_owned_acl(&sandbox.root, &user, None)?;
        fs::create_dir(&sandbox.stage)?;
        fs::create_dir(&sandbox.controls)?;
        let mut output = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&sandbox.binary)?;
        let copied = io::copy(
            &mut Read::by_ref(&mut input).take(64 * 1024 * 1024 + 1),
            &mut output,
        )?;
        if copied != metadata.len() {
            return Err(io::Error::other(
                "trusted executable changed during staging",
            ));
        }
        output.sync_all()?;
        drop(output);
        let mut fixture = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&sandbox.fixture)?;
        fixture.write_all(MARKER)?;
        drop(fixture);
        let owner_bytes = [OWNER_MARKER, name.as_bytes(), b"\n"].concat();
        if owner_bytes.len() > MAX_OWNER_BYTES {
            return Err(io::Error::other("sandbox owner marker exceeds bound"));
        }
        let mut owner = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&sandbox.owner)?;
        owner.write_all(&owner_bytes)?;
        drop(owner);
        set_owned_acl(&sandbox.fixture, &user, None)?;
        set_owned_acl(&sandbox.controls, &user, None)?;
        set_owned_acl(&sandbox.owner, &user, None)?;
        set_owned_acl(&sandbox.binary, &user, Some(&sandbox.sid))?;
        set_owned_acl(&sandbox.stage, &user, Some(&sandbox.sid))?;
        // The run root grants traverse/read only. The sibling controls directory
        // remains user/SYSTEM-only, so no deny-fixture right is inherited.
        set_owned_acl(&sandbox.root, &user, Some(&sandbox.sid))?;
        Ok(sandbox)
    }

    pub fn executable(&self) -> &Path {
        &self.binary
    }
    pub fn root(&self) -> &Path {
        &self.root
    }
    pub fn fixture(&self) -> &Path {
        &self.fixture
    }
    pub(crate) fn capabilities(&self) -> SECURITY_CAPABILITIES {
        SECURITY_CAPABILITIES {
            AppContainerSid: self.sid.pointer(),
            Capabilities: ptr::null_mut(),
            CapabilityCount: 0,
            Reserved: 0,
        }
    }

    /// Recover only a generated run whose private owner receipt names the same
    /// fresh profile. Used after the real parent-death probe bypasses Drop.
    pub fn cleanup_abandoned(root: &Path) -> io::Result<()> {
        if !root.is_absolute() {
            return Err(io::Error::other("abandoned staging must be absolute"));
        }
        let name = root
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| io::Error::other("invalid abandoned profile name"))?;
        if !valid_generated_name(name) {
            return Err(io::Error::other("unowned abandoned profile name"));
        }
        let runs = root
            .parent()
            .filter(|path| path.file_name() == Some(std::ffi::OsStr::new("runs")))
            .ok_or_else(|| io::Error::other("abandoned staging is outside runs"))?;
        let sandbox4 = runs
            .parent()
            .filter(|path| path.file_name() == Some(std::ffi::OsStr::new("sandbox4")))
            .ok_or_else(|| io::Error::other("abandoned staging is outside sandbox4"))?;
        if sandbox4.parent().and_then(Path::file_name) != Some(std::ffi::OsStr::new("target")) {
            return Err(io::Error::other(
                "abandoned staging is outside target/sandbox4",
            ));
        }
        let canonical_runs = runs.canonicalize()?;
        let canonical = root.canonicalize()?;
        if canonical.parent() != Some(canonical_runs.as_path()) {
            return Err(io::Error::other(
                "abandoned staging escaped its owned parent",
            ));
        }
        reject_reparse(root)?;
        reject_reparse(&root.join("owner.txt"))?;
        let owner = OpenOptions::new()
            .read(true)
            .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT)
            .open(root.join("owner.txt"))?;
        let metadata = owner.metadata()?;
        use std::os::windows::fs::MetadataExt;
        if !metadata.is_file()
            || metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
            || metadata.len() > MAX_OWNER_BYTES as u64
        {
            return Err(io::Error::other("invalid abandoned owner receipt"));
        }
        let mut bytes = Vec::new();
        owner
            .take(MAX_OWNER_BYTES as u64 + 1)
            .read_to_end(&mut bytes)?;
        let expected = [OWNER_MARKER, name.as_bytes(), b"\n"].concat();
        if bytes != expected {
            return Err(io::Error::other("abandoned owner receipt mismatch"));
        }
        let name_wide: Vec<u16> = name.encode_utf16().chain(Some(0)).collect();
        // SAFETY: the bounded receipt and canonical generated root bind this exact
        // fresh profile name; no collision/derived/arbitrary profile is deleted.
        let result = unsafe { DeleteAppContainerProfile(name_wide.as_ptr()) };
        if result < 0 {
            return Err(io::Error::other(format!(
                "abandoned AppContainer cleanup failed HRESULT={result:#x}"
            )));
        }
        remove_owned_staging(root, &canonical)
    }

    pub fn cleanup(mut self) -> io::Result<()> {
        let root = self.root().to_path_buf();
        let result = Self::cleanup_abandoned(&root);
        if result.is_ok() {
            self.profile_created = false;
            self.directory_created = false;
        }
        result
    }

    fn cleanup_inner(&mut self) -> io::Result<()> {
        if self.profile_created {
            // SAFETY: only successful fresh creation sets this ownership flag;
            // bounded name is never derived/reused from another user's profile.
            let result = unsafe { DeleteAppContainerProfile(self.name.as_ptr()) };
            if result < 0 {
                return Err(io::Error::other(format!(
                    "AppContainer cleanup failed HRESULT={result:#x}"
                )));
            }
            self.profile_created = false;
        }
        if self.directory_created {
            let canonical = self
                .canonical_root
                .as_deref()
                .ok_or_else(|| io::Error::other("missing staging identity"))?;
            remove_owned_staging(&self.root, canonical)?;
            self.directory_created = false;
        }
        Ok(())
    }
}

impl Drop for RendererSandbox {
    fn drop(&mut self) {
        let _ = self.cleanup_inner();
    }
}

fn local_drive_path(path: &Path) -> io::Result<PathBuf> {
    let text = path
        .to_str()
        .ok_or_else(|| io::Error::other("sandbox staging path must be Unicode"))?;
    let Some(verbatim) = text.strip_prefix(r"\\?\") else {
        return Ok(path.to_path_buf());
    };
    let bytes = verbatim.as_bytes();
    if bytes.len() < 3
        || !bytes[0].is_ascii_alphabetic()
        || bytes[1] != b':'
        || !matches!(bytes[2], b'/' | b'\\')
    {
        return Err(io::Error::other("sandbox staging requires a local drive"));
    }
    Ok(PathBuf::from(verbatim))
}

fn valid_generated_name(name: &str) -> bool {
    let Some(suffix) = name.strip_prefix("OpenSequoia.Probe.") else {
        return false;
    };
    let parts: Vec<_> = suffix.split('.').collect();
    name.len() <= 64
        && parts.len() == 3
        && parts
            .iter()
            .all(|part| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_hexdigit()))
}

fn reject_reparse(path: &Path) -> io::Result<()> {
    use std::os::windows::fs::MetadataExt;
    if fs::symlink_metadata(path)?.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
        return Err(io::Error::other("sandbox cleanup found a reparse point"));
    }
    Ok(())
}

fn reject_reparse_if_exists(path: &Path) -> io::Result<()> {
    use std::os::windows::fs::MetadataExt;
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 => {
            Err(io::Error::other("sandbox cleanup found a reparse point"))
        }
        Ok(_) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

fn remove_owned_staging(root: &Path, expected_canonical: &Path) -> io::Result<()> {
    if root.canonicalize()? != expected_canonical {
        return Err(io::Error::other("sandbox staging identity changed"));
    }
    let stage = root.join("stage");
    let controls = root.join("controls");
    let binary = stage.join("renderer.exe");
    let fixture = controls.join("deny.dat");
    let owner = root.join("owner.txt");
    for path in [root, &stage, &controls, &binary, &fixture, &owner] {
        reject_reparse_if_exists(path)?;
    }
    for path in [&binary, &fixture, &owner] {
        match fs::remove_file(path) {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
    }
    // No recursive deletion: unexpected extra entries preserve the run root and
    // produce a visible error instead of deleting unrecognized data.
    for path in [&stage, &controls, root] {
        match fs::remove_dir(path) {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
    }
    Ok(())
}

/// Measured from the exact owned, suspended process token before execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TokenPolicy {
    pub appcontainer: bool,
    pub integrity_rid: u32,
    pub capability_count: u32,
}

pub(crate) fn verify_process(
    process: HANDLE,
    sandbox: &RendererSandbox,
) -> io::Result<TokenPolicy> {
    let token = process_token(process)?;
    if !is_appcontainer(token.as_raw_handle())? {
        return Err(io::Error::other("child is not an AppContainer"));
    }
    let app = TokenBuffer::query(token.as_raw_handle(), TokenAppContainerSid)?;
    if app.bytes < size_of::<TOKEN_APPCONTAINER_INFORMATION>() {
        return Err(io::Error::other("truncated AppContainer token"));
    }
    // SAFETY: checked initialized C structure extent, pointers validated against
    // the token allocation before SID APIs inspect them.
    let identity = unsafe {
        ptr::read_unaligned(app.data().as_ptr().cast::<TOKEN_APPCONTAINER_INFORMATION>())
    };
    let identity = app.sid(identity.TokenAppContainer)?;
    // SAFETY: both complete validated owned SID allocations remain live.
    if unsafe { EqualSid(identity.pointer(), sandbox.sid.pointer()) } == 0 {
        return Err(io::Error::other("AppContainer identity mismatch"));
    }
    let integrity = TokenBuffer::query(token.as_raw_handle(), TokenIntegrityLevel)?;
    if integrity.bytes < size_of::<TOKEN_MANDATORY_LABEL>() {
        return Err(io::Error::other("truncated integrity label"));
    }
    // SAFETY: complete C integer/pointer structure; pointed SID bounds checked.
    let label =
        unsafe { ptr::read_unaligned(integrity.data().as_ptr().cast::<TOKEN_MANDATORY_LABEL>()) };
    let label = integrity.sid(label.Label.Sid)?;
    let integrity_rid = label
        .low_integrity_rid()
        .ok_or_else(|| io::Error::other("child integrity label is not Low"))?;
    if integrity_rid != 4096 {
        return Err(io::Error::other(
            "child integrity is not exact Low RID 4096",
        ));
    }
    let caps = TokenBuffer::query(token.as_raw_handle(), TokenCapabilities)?;
    let count = u32::from_le_bytes(
        caps.data()
            .get(..4)
            .ok_or_else(|| io::Error::other("truncated capability count"))?
            .try_into()
            .map_err(|_| io::Error::other("invalid capability count"))?,
    );
    if count != 0 {
        return Err(io::Error::other("unexpected AppContainer capabilities"));
    }
    Ok(TokenPolicy {
        appcontainer: true,
        integrity_rid,
        capability_count: count,
    })
}

/// Synthetic denial worker; never logs input contents or connects off-host.
/// The ordinary control rewrites identical marker bytes; AppContainer must fail
/// opening the same user/SYSTEM-only fixture for both read and write with error 5.
pub fn denial_worker(path: &Path, port: u16) -> io::Result<()> {
    let result = denial_worker_inner(path, port);
    if let Err(error) = &result {
        // Bounded by output_child and contains only operation/OS error data, not
        // fixture bytes, environment, credentials or browsing information.
        println!("SANDBOX_PROBE_ERROR operation={error}");
    }
    result
}

fn denial_worker_inner(path: &Path, port: u16) -> io::Result<()> {
    if port == 0
        || !path.is_absolute()
        || path.file_name() != Some(std::ffi::OsStr::new("deny.dat"))
    {
        return Err(io::Error::other("invalid sandbox probe arguments"));
    }
    let controls = path
        .parent()
        .filter(|path| path.file_name() == Some(std::ffi::OsStr::new("controls")))
        .ok_or_else(|| io::Error::other("probe fixture is outside controls"))?;
    let root = controls
        .parent()
        .ok_or_else(|| io::Error::other("missing probe run directory"))?;
    let name = root
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| io::Error::other("invalid probe run directory"))?;
    if !valid_generated_name(name)
        || root.parent().and_then(Path::file_name) != Some(std::ffi::OsStr::new("runs"))
        || root
            .parent()
            .and_then(Path::parent)
            .and_then(Path::file_name)
            != Some(std::ffi::OsStr::new("sandbox4"))
        || root
            .parent()
            .and_then(Path::parent)
            .and_then(Path::parent)
            .and_then(Path::file_name)
            != Some(std::ffi::OsStr::new("target"))
    {
        return Err(io::Error::other("unowned probe directory"));
    }
    let mut content = Vec::new();
    let read = match OpenOptions::new()
        .read(true)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT)
        .open(path)
    {
        Ok(file) => {
            use std::os::windows::fs::MetadataExt;
            if file.metadata()?.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
                return Err(io::Error::other("probe fixture is reparse point"));
            }
            file.take(MARKER.len() as u64 + 1)
                .read_to_end(&mut content)?;
            if content != MARKER {
                return Err(io::Error::other("probe fixture marker mismatch"));
            }
            0
        }
        Err(error) if error.raw_os_error() == Some(5) => 5,
        Err(error) => return Err(io::Error::other(format!("probe read failed: {error}"))),
    };
    let write = match OpenOptions::new()
        .write(true)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT)
        .open(path)
    {
        Ok(mut file) => {
            use std::os::windows::fs::MetadataExt;
            if file.metadata()?.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
                return Err(io::Error::other("probe fixture is reparse point"));
            }
            file.write_all(MARKER)?;
            file.sync_all()?;
            0
        }
        Err(error) if error.raw_os_error() == Some(5) => 5,
        Err(error) => return Err(io::Error::other(format!("probe write failed: {error}"))),
    };
    // SAFETY: borrowed documented current-process pseudo handle.
    let token = process_token(unsafe { GetCurrentProcess() })?;
    let app = u8::from(is_appcontainer(token.as_raw_handle())?);
    println!("SANDBOX_PROBE_FILES app={app} read={read} write={write}");
    let network = winsock_loopback(port)?;
    println!("SANDBOX_PROBE app={app} read={read} write={write} network={network}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn malformed_sid_sizes_and_foreign_token_pointers_are_rejected() {
        for size in [0, 7, 9, 69] {
            assert!(Sid::copy_validated(&vec![0; size]).is_err());
        }
        let mut bytes = [0u8; 8];
        bytes[0] = 1;
        bytes[1] = 16;
        assert!(Sid::copy_validated(&bytes).is_err());
        let buffer = TokenBuffer {
            words: vec![0; 4],
            bytes: 32,
        };
        assert!(buffer.sid(ptr::null_mut()).is_err());
        assert!(buffer.sid(usize::MAX as PSID).is_err());
        assert!(
            buffer
                .sid((buffer.words.as_ptr() as usize + 30) as PSID)
                .is_err()
        );
    }

    #[test]
    fn integrity_sid_parser_distinguishes_low_medium_and_wrong_authority() {
        // Value: protects=exact Low token policy; fails_when=RID/authority parsing regresses;
        // why_new=malformed SID extent tests do not cover integrity semantics;
        // seam=none; no_claim=a synthetic SID substitutes for the real child-token check.
        let low = Sid::copy_validated(&[1, 1, 0, 0, 0, 0, 0, 16, 0, 16, 0, 0]).unwrap();
        assert_eq!(low.low_integrity_rid(), Some(4096));
        let medium = Sid::copy_validated(&[1, 1, 0, 0, 0, 0, 0, 16, 0, 32, 0, 0]).unwrap();
        assert_ne!(medium.low_integrity_rid(), Some(4096));
        let wrong_authority = Sid::copy_validated(&[1, 1, 0, 0, 0, 0, 0, 5, 0, 16, 0, 0]).unwrap();
        assert_eq!(wrong_authority.low_integrity_rid(), None);
    }

    #[test]
    fn ordinary_process_token_query_and_user_sid_are_real() {
        let user = token_user().unwrap();
        assert!((8..=MAX_SID_BYTES).contains(&user.bytes));
        let token = process_token(unsafe { GetCurrentProcess() }).unwrap();
        assert!(!is_appcontainer(token.as_raw_handle()).unwrap());
    }

    #[test]
    fn denial_helper_rejects_unowned_paths_before_io() {
        assert!(denial_worker(Path::new("C:/ordinary-user-data/deny.dat"), 123).is_err());
        assert!(denial_worker(Path::new("relative/deny.dat"), 123).is_err());
        assert!(denial_worker(Path::new("C:/OpenSequoia.Probe.a.b.c/deny.dat"), 0).is_err());
    }
}
