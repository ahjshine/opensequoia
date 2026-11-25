#![forbid(unsafe_code)]

//! Version 2 bounded local renderer frames. This transport provides no peer authentication,
//! sandbox, origin identity or capability checks. Those remain broker/security work.

use std::fmt;
use std::io::{self, Read, Write};

use opensequoia_engine::DisplayList;

mod codec;

pub const HEADER_BYTES: usize = 16;
pub const MAX_FRAME_BYTES: usize = 4 * 1024 * 1024;
pub const MAX_RUNS: usize = 65_536;
pub const MAX_TOTAL_TEXT_BYTES: usize = 1024 * 1024;
pub const MAX_ERROR_BYTES: usize = 1024;
pub const VERSION: u16 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    Parse,
    Layout,
    Protocol,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Message {
    Render { html: String, width: u32 },
    Display(DisplayList),
    Error { code: ErrorCode, message: String },
}

#[derive(Debug)]
pub enum IpcError {
    Io(io::Error),
    Truncated,
    BadMagic,
    UnsupportedVersion(u16),
    UnknownTag(u8),
    Reserved,
    FrameTooLarge,
    InvalidUtf8,
    InvalidViewport,
    InvalidDisplay,
    InvalidErrorCode,
    ErrorTooLarge,
    TrailingPayload,
}

impl fmt::Display for IpcError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for IpcError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            _ => None,
        }
    }
}

/// Encode one validated frame. Validation runs before allocating or writing frame bytes.
pub fn encode(message: &Message) -> Result<Vec<u8>, IpcError> {
    codec::encode(message)
}

/// Decode exactly one complete frame; extra bytes are rejected, never ignored.
pub fn decode(frame: &[u8]) -> Result<Message, IpcError> {
    codec::decode(frame)
}

/// Read one frame from a stream. It leaves subsequent frames unread. Invalid frames are
/// connection-fatal; callers must not attempt to resume parsing or reuse the connection.
/// Read deadlines, cancellation, peer identity and process isolation belong to the caller.
pub fn read_message<R: Read + ?Sized>(reader: &mut R) -> Result<Message, IpcError> {
    codec::read_message(reader)
}

/// Validate and write one frame. On IO failure the stream may contain a partial frame and
/// must be discarded; this function does not retry an application request or flush a writer.
pub fn write_message<W: Write + ?Sized>(writer: &mut W, message: &Message) -> Result<(), IpcError> {
    writer.write_all(&encode(message)?).map_err(IpcError::Io)
}
