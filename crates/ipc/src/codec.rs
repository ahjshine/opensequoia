//! Wire format v2, all integers little endian and strings exact-length UTF-8.
//!
//! Header (16 bytes): `SQIP` magic[4], version:u16=2, tag:u8, reserved:u8=0,
//! payload_bytes:u32, reserved:u32=0. Total frame <=4 MiB, including header.
//! Tag 1 Render: viewport_width:u32, html_bytes:u32, html[html_bytes].
//! Tag 2 Display: width:u32, height:u32, count:u32, then `count` tagged items:
//! tag=1 text: x:i32,y:i32,width:u32,height:u32,font_size:u32,bold:u8(0/1),
//! r:u8,g:u8,b:u8,text_bytes:u32,text[text_bytes].
//! tag=2 fill: x:i32,y:i32,width:u32,height:u32,r:u8,g:u8,b:u8.
//! No implicit padding or native Rust layout.
//! Tag 3 Error: code:u16(1 Parse/2 Layout/3 Protocol), reserved:u16=0,
//! message_bytes:u32,message[message_bytes]. Every payload must end exactly.

use std::io::{self, Read};

use opensequoia_engine::{Color, DisplayItem, DisplayList, FillRect, MAX_INPUT_BYTES, TextRun};

use crate::{
    ErrorCode, HEADER_BYTES, IpcError, MAX_ERROR_BYTES, MAX_FRAME_BYTES, MAX_RUNS,
    MAX_TOTAL_TEXT_BYTES, Message, VERSION,
};

const MAGIC: &[u8; 4] = b"SQIP";
const MAX_HEIGHT: u32 = 1_000_000;
const TEXT_TAG: u8 = 1;
const FILL_TAG: u8 = 2;
const TEXT_ITEM_BYTES: usize = 29;
const FILL_ITEM_BYTES: usize = 20;
const MIN_ITEM_BYTES: usize = 20;

fn valid_viewport(width: u32) -> Result<(), IpcError> {
    if !(64..=8192).contains(&width) {
        return Err(IpcError::InvalidViewport);
    }
    Ok(())
}

fn valid_box(list_width: u32, list_height: u32, x: i32, y: i32, width: u32, height: u32) -> bool {
    x >= 0
        && y >= 0
        && width > 0
        && height > 0
        && (x as u32)
            .checked_add(width)
            .is_some_and(|end| end <= list_width)
        && (y as u32)
            .checked_add(height)
            .is_some_and(|end| end <= list_height)
}

fn valid_text(
    list_width: u32,
    list_height: u32,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    font_size: u32,
) -> bool {
    (1..=256).contains(&font_size) && valid_box(list_width, list_height, x, y, width, height)
}

fn validate(message: &Message) -> Result<(u8, usize), IpcError> {
    let (tag, length) = match message {
        Message::Render { html, width } => {
            valid_viewport(*width)?;
            if html.len() > MAX_INPUT_BYTES {
                return Err(IpcError::FrameTooLarge);
            }
            (1, 8 + html.len())
        }
        Message::Display(list) => {
            valid_viewport(list.width)?;
            if list.height == 0 || list.height > MAX_HEIGHT || list.items.len() > MAX_RUNS {
                return Err(IpcError::InvalidDisplay);
            }
            let mut total_text = 0_usize;
            let mut length = 12_usize;
            for item in &list.items {
                match item {
                    DisplayItem::Text(run) => {
                        if run.text.is_empty()
                            || run.text.len() > MAX_INPUT_BYTES
                            || !valid_text(
                                list.width,
                                list.height,
                                run.x,
                                run.y,
                                run.width,
                                run.height,
                                run.font_size,
                            )
                        {
                            return Err(IpcError::InvalidDisplay);
                        }
                        total_text = total_text
                            .checked_add(run.text.len())
                            .ok_or(IpcError::InvalidDisplay)?;
                        if total_text > MAX_TOTAL_TEXT_BYTES {
                            return Err(IpcError::InvalidDisplay);
                        }
                        length = length
                            .checked_add(TEXT_ITEM_BYTES + run.text.len())
                            .ok_or(IpcError::InvalidDisplay)?;
                    }
                    DisplayItem::Fill(rect) => {
                        if !valid_box(
                            list.width,
                            list.height,
                            rect.x,
                            rect.y,
                            rect.width,
                            rect.height,
                        ) {
                            return Err(IpcError::InvalidDisplay);
                        }
                        length = length
                            .checked_add(FILL_ITEM_BYTES)
                            .ok_or(IpcError::InvalidDisplay)?;
                    }
                }
            }
            (2, length)
        }
        Message::Error { message, .. } => {
            if message.len() > MAX_ERROR_BYTES {
                return Err(IpcError::ErrorTooLarge);
            }
            (3, 8 + message.len())
        }
    };
    if length > MAX_FRAME_BYTES - HEADER_BYTES {
        return Err(IpcError::FrameTooLarge);
    }
    Ok((tag, length))
}

fn push_u32(bytes: &mut Vec<u8>, value: u32) {
    bytes.extend_from_slice(&value.to_le_bytes());
}
fn push_string(bytes: &mut Vec<u8>, value: &str) {
    // validate() proved every byte length is <=1 MiB and thus fits u32.
    push_u32(bytes, value.len() as u32);
    bytes.extend_from_slice(value.as_bytes());
}

pub(crate) fn encode(message: &Message) -> Result<Vec<u8>, IpcError> {
    let (tag, length) = validate(message)?;
    let mut bytes = Vec::with_capacity(HEADER_BYTES + length);
    bytes.extend_from_slice(MAGIC);
    bytes.extend_from_slice(&VERSION.to_le_bytes());
    bytes.push(tag);
    bytes.push(0);
    push_u32(&mut bytes, length as u32);
    push_u32(&mut bytes, 0);
    match message {
        Message::Render { html, width } => {
            push_u32(&mut bytes, *width);
            push_string(&mut bytes, html);
        }
        Message::Display(list) => {
            push_u32(&mut bytes, list.width);
            push_u32(&mut bytes, list.height);
            push_u32(&mut bytes, list.items.len() as u32);
            for item in &list.items {
                match item {
                    DisplayItem::Text(run) => {
                        bytes.push(TEXT_TAG);
                        bytes.extend_from_slice(&run.x.to_le_bytes());
                        bytes.extend_from_slice(&run.y.to_le_bytes());
                        push_u32(&mut bytes, run.width);
                        push_u32(&mut bytes, run.height);
                        push_u32(&mut bytes, run.font_size);
                        bytes.push(u8::from(run.bold));
                        bytes.extend_from_slice(&[run.color.r, run.color.g, run.color.b]);
                        push_string(&mut bytes, &run.text);
                    }
                    DisplayItem::Fill(rect) => {
                        bytes.push(FILL_TAG);
                        bytes.extend_from_slice(&rect.x.to_le_bytes());
                        bytes.extend_from_slice(&rect.y.to_le_bytes());
                        push_u32(&mut bytes, rect.width);
                        push_u32(&mut bytes, rect.height);
                        bytes.extend_from_slice(&[rect.color.r, rect.color.g, rect.color.b]);
                    }
                }
            }
        }
        Message::Error { code, message } => {
            let code: u16 = match code {
                ErrorCode::Parse => 1,
                ErrorCode::Layout => 2,
                ErrorCode::Protocol => 3,
            };
            bytes.extend_from_slice(&code.to_le_bytes());
            bytes.extend_from_slice(&0_u16.to_le_bytes());
            push_string(&mut bytes, message);
        }
    }
    Ok(bytes)
}

fn header(bytes: &[u8]) -> Result<(u8, usize), IpcError> {
    if bytes.len() != HEADER_BYTES {
        return Err(IpcError::Truncated);
    }
    if &bytes[..4] != MAGIC {
        return Err(IpcError::BadMagic);
    }
    let version = u16::from_le_bytes([bytes[4], bytes[5]]);
    if version != VERSION {
        return Err(IpcError::UnsupportedVersion(version));
    }
    if bytes[7] != 0 || bytes[12..16] != [0, 0, 0, 0] {
        return Err(IpcError::Reserved);
    }
    let tag = bytes[6];
    let length = u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]) as usize;
    let (minimum, maximum) = match tag {
        1 => (8, 8 + MAX_INPUT_BYTES),
        2 => (12, 12 + MAX_RUNS * TEXT_ITEM_BYTES + MAX_TOTAL_TEXT_BYTES),
        3 => (8, 8 + MAX_ERROR_BYTES),
        _ => return Err(IpcError::UnknownTag(tag)),
    };
    if length > MAX_FRAME_BYTES - HEADER_BYTES || length > maximum {
        return Err(IpcError::FrameTooLarge);
    }
    if length < minimum {
        return Err(IpcError::Truncated);
    }
    Ok((tag, length))
}

fn read_exact<R: Read + ?Sized>(reader: &mut R, bytes: &mut [u8]) -> Result<(), IpcError> {
    reader.read_exact(bytes).map_err(|error| {
        if error.kind() == io::ErrorKind::UnexpectedEof {
            IpcError::Truncated
        } else {
            IpcError::Io(error)
        }
    })
}

pub(crate) fn read_message<R: Read + ?Sized>(reader: &mut R) -> Result<Message, IpcError> {
    let mut bytes = [0_u8; HEADER_BYTES];
    read_exact(reader, &mut bytes)?;
    let (tag, length) = header(&bytes)?;
    // Validated magic/version/type/reserved/length bounds precede payload allocation/read.
    let mut payload = vec![0_u8; length];
    read_exact(reader, &mut payload)?;
    payload_message(tag, &payload)
}

pub(crate) fn decode(frame: &[u8]) -> Result<Message, IpcError> {
    let (tag, length) = header(frame.get(..HEADER_BYTES).ok_or(IpcError::Truncated)?)?;
    let end = HEADER_BYTES + length;
    if frame.len() < end {
        return Err(IpcError::Truncated);
    }
    if frame.len() > end {
        return Err(IpcError::TrailingPayload);
    }
    payload_message(tag, &frame[HEADER_BYTES..end])
}

struct Payload<'a> {
    bytes: &'a [u8],
    position: usize,
}
impl<'a> Payload<'a> {
    fn remaining(&self) -> usize {
        self.bytes.len() - self.position
    }
    fn take(&mut self, length: usize) -> Result<&'a [u8], IpcError> {
        let end = self
            .position
            .checked_add(length)
            .ok_or(IpcError::Truncated)?;
        let value = self
            .bytes
            .get(self.position..end)
            .ok_or(IpcError::Truncated)?;
        self.position = end;
        Ok(value)
    }
    fn u32(&mut self) -> Result<u32, IpcError> {
        let b = self.take(4)?;
        Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }
    fn u16(&mut self) -> Result<u16, IpcError> {
        let b = self.take(2)?;
        Ok(u16::from_le_bytes([b[0], b[1]]))
    }
    fn string(&mut self, length: usize) -> Result<String, IpcError> {
        Ok(std::str::from_utf8(self.take(length)?)
            .map_err(|_| IpcError::InvalidUtf8)?
            .to_owned())
    }
}

fn payload_message(tag: u8, bytes: &[u8]) -> Result<Message, IpcError> {
    let mut payload = Payload { bytes, position: 0 };
    let message = match tag {
        1 => {
            let width = payload.u32()?;
            valid_viewport(width)?;
            let length = payload.u32()? as usize;
            if length > MAX_INPUT_BYTES {
                return Err(IpcError::FrameTooLarge);
            }
            Message::Render {
                width,
                html: payload.string(length)?,
            }
        }
        2 => {
            let width = payload.u32()?;
            valid_viewport(width)?;
            let height = payload.u32()?;
            let count = payload.u32()? as usize;
            if height == 0 || height > MAX_HEIGHT || count > MAX_RUNS {
                return Err(IpcError::InvalidDisplay);
            }
            if count
                .checked_mul(MIN_ITEM_BYTES)
                .is_none_or(|minimum| minimum > payload.remaining())
            {
                return Err(IpcError::Truncated);
            }
            let mut items = Vec::with_capacity(count);
            let mut total_text = 0_usize;
            for _ in 0..count {
                match payload.take(1)?[0] {
                    TEXT_TAG => {
                        let x = payload.u32()? as i32;
                        let y = payload.u32()? as i32;
                        let run_width = payload.u32()?;
                        let run_height = payload.u32()?;
                        let font_size = payload.u32()?;
                        let flags = payload.take(4)?;
                        if flags[0] > 1
                            || !valid_text(width, height, x, y, run_width, run_height, font_size)
                        {
                            return Err(IpcError::InvalidDisplay);
                        }
                        let bold = flags[0] == 1;
                        let color = Color {
                            r: flags[1],
                            g: flags[2],
                            b: flags[3],
                        };
                        let length = payload.u32()? as usize;
                        total_text = total_text
                            .checked_add(length)
                            .ok_or(IpcError::InvalidDisplay)?;
                        if length == 0
                            || length > MAX_INPUT_BYTES
                            || total_text > MAX_TOTAL_TEXT_BYTES
                        {
                            return Err(IpcError::InvalidDisplay);
                        }
                        let text = payload.string(length)?;
                        items.push(DisplayItem::Text(TextRun {
                            text,
                            x,
                            y,
                            width: run_width,
                            height: run_height,
                            font_size,
                            bold,
                            color,
                        }));
                    }
                    FILL_TAG => {
                        let x = payload.u32()? as i32;
                        let y = payload.u32()? as i32;
                        let fill_width = payload.u32()?;
                        let fill_height = payload.u32()?;
                        let rgb = payload.take(3)?;
                        if !valid_box(width, height, x, y, fill_width, fill_height) {
                            return Err(IpcError::InvalidDisplay);
                        }
                        items.push(DisplayItem::Fill(FillRect {
                            x,
                            y,
                            width: fill_width,
                            height: fill_height,
                            color: Color {
                                r: rgb[0],
                                g: rgb[1],
                                b: rgb[2],
                            },
                        }));
                    }
                    tag => return Err(IpcError::UnknownTag(tag)),
                }
            }
            Message::Display(DisplayList {
                width,
                height,
                items,
            })
        }
        3 => {
            let code = match payload.u16()? {
                1 => ErrorCode::Parse,
                2 => ErrorCode::Layout,
                3 => ErrorCode::Protocol,
                _ => return Err(IpcError::InvalidErrorCode),
            };
            if payload.u16()? != 0 {
                return Err(IpcError::Reserved);
            }
            let length = payload.u32()? as usize;
            if length > MAX_ERROR_BYTES {
                return Err(IpcError::ErrorTooLarge);
            }
            Message::Error {
                code,
                message: payload.string(length)?,
            }
        }
        _ => return Err(IpcError::UnknownTag(tag)),
    };
    if payload.remaining() != 0 {
        return Err(IpcError::TrailingPayload);
    }
    Ok(message)
}
