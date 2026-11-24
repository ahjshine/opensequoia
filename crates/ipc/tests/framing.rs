use std::io::{self, Cursor, Read, Write};

use opensequoia_engine::{Color, DisplayItem, DisplayList, MAX_INPUT_BYTES, TextRun};
use opensequoia_ipc::{
    ErrorCode, HEADER_BYTES, IpcError, MAX_ERROR_BYTES, MAX_FRAME_BYTES, MAX_RUNS,
    MAX_TOTAL_TEXT_BYTES, Message, decode, encode, read_message, write_message,
};

fn request() -> Message {
    Message::Render {
        html: "<p>नमस्ते &amp; 世界</p>".into(),
        width: 800,
    }
}
fn display() -> Message {
    Message::Display(DisplayList {
        width: 800,
        height: 80,
        items: vec![DisplayItem::Text(TextRun {
            text: "Hello 世界".into(),
            x: 16,
            y: 16,
            width: 80,
            height: 24,
            font_size: 16,
            bold: false,
            color: Color {
                r: 28,
                g: 41,
                b: 35,
            },
        })],
    })
}
fn header(tag: u8, payload: u32) -> Vec<u8> {
    let mut bytes = b"SQIP".to_vec();
    bytes.extend_from_slice(&1_u16.to_le_bytes());
    bytes.push(tag);
    bytes.push(0);
    bytes.extend_from_slice(&payload.to_le_bytes());
    bytes.extend_from_slice(&0_u32.to_le_bytes());
    bytes
}

#[test]
fn round_trip_all_message_types_and_streamed_frames() {
    for message in [
        request(),
        display(),
        Message::Error {
            code: ErrorCode::Layout,
            message: "layout सीमा".into(),
        },
    ] {
        let bytes = encode(&message).unwrap();
        assert_eq!(decode(&bytes).unwrap(), message);
        let mut stream = Vec::new();
        write_message(&mut stream, &message).unwrap();
        assert_eq!(read_message(&mut Cursor::new(stream)).unwrap(), message);
    }
}

#[test]
fn magic_version_tags_and_reserved_fields_fail_closed() {
    let bytes = encode(&request()).unwrap();
    for (offset, value) in [(0, b'X'), (4, 2), (6, 99), (7, 1), (12, 1)] {
        let mut malformed = bytes.clone();
        malformed[offset] = value;
        assert!(decode(&malformed).is_err(), "offset {offset} accepted");
    }
}

struct HeaderOnly {
    header: Cursor<Vec<u8>>,
    payload_reads: usize,
}
impl Read for HeaderOnly {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        if self.header.position() as usize == HEADER_BYTES {
            self.payload_reads += 1;
            return Err(io::Error::other("payload must not be read"));
        }
        self.header.read(buffer)
    }
}

#[test]
fn oversized_or_type_invalid_header_is_rejected_before_payload_read() {
    for bytes in [
        header(2, MAX_FRAME_BYTES as u32),
        header(1, (MAX_INPUT_BYTES + 9) as u32),
        header(3, (MAX_ERROR_BYTES + 9) as u32),
        header(99, 0),
    ] {
        let mut reader = HeaderOnly {
            header: Cursor::new(bytes),
            payload_reads: 0,
        };
        assert!(read_message(&mut reader).is_err());
        assert_eq!(reader.payload_reads, 0);
    }
}

#[test]
fn every_truncated_prefix_non_utf8_and_trailing_data_is_rejected() {
    for message in [request(), display()] {
        let bytes = encode(&message).unwrap();
        for length in 0..bytes.len() {
            assert!(decode(&bytes[..length]).is_err());
        }
        let mut trailing = bytes.clone();
        trailing.push(0);
        assert!(matches!(decode(&trailing), Err(IpcError::TrailingPayload)));
    }
    let mut bytes = encode(&request()).unwrap();
    bytes[HEADER_BYTES + 8] = 0xff;
    assert!(matches!(decode(&bytes), Err(IpcError::InvalidUtf8)));
    let mut bytes = encode(&request()).unwrap();
    bytes.push(b'x');
    let payload = (bytes.len() - HEADER_BYTES) as u32;
    bytes[8..12].copy_from_slice(&payload.to_le_bytes());
    assert!(matches!(decode(&bytes), Err(IpcError::TrailingPayload)));
}

#[test]
fn request_and_error_encode_bounds_do_not_truncate() {
    for width in [0, 63, 8193, u32::MAX] {
        assert!(matches!(
            encode(&Message::Render {
                html: String::new(),
                width
            }),
            Err(IpcError::InvalidViewport)
        ));
    }
    assert!(
        encode(&Message::Render {
            html: "x".repeat(MAX_INPUT_BYTES),
            width: 64
        })
        .is_ok()
    );
    assert!(
        encode(&Message::Render {
            html: "x".repeat(MAX_INPUT_BYTES + 1),
            width: 64
        })
        .is_err()
    );
    assert!(
        encode(&Message::Error {
            code: ErrorCode::Parse,
            message: "x".repeat(MAX_ERROR_BYTES)
        })
        .is_ok()
    );
    assert!(matches!(
        encode(&Message::Error {
            code: ErrorCode::Parse,
            message: "x".repeat(MAX_ERROR_BYTES + 1)
        }),
        Err(IpcError::ErrorTooLarge)
    ));
}

#[test]
fn display_bounds_and_boolean_wire_field_are_validated() {
    for mutate in [0, 1, 2, 3, 4, 5, 6] {
        let Message::Display(mut list) = display() else {
            unreachable!()
        };
        let DisplayItem::Text(run) = &mut list.items[0];
        match mutate {
            0 => run.x = -1,
            1 => run.y = -1,
            2 => run.width = u32::MAX,
            3 => run.height = 1000,
            4 => run.font_size = 0,
            5 => run.font_size = 257,
            6 => run.text = "x".repeat(MAX_INPUT_BYTES + 1),
            _ => unreachable!(),
        }
        assert!(matches!(
            encode(&Message::Display(list)),
            Err(IpcError::InvalidDisplay)
        ));
    }
    let mut bytes = encode(&display()).unwrap();
    bytes[HEADER_BYTES + 12 + 20] = 2;
    assert!(matches!(decode(&bytes), Err(IpcError::InvalidDisplay)));
    let mut bytes = encode(&display()).unwrap();
    bytes[HEADER_BYTES + 8..HEADER_BYTES + 12]
        .copy_from_slice(&((MAX_RUNS + 1) as u32).to_le_bytes());
    assert!(decode(&bytes).is_err());
}

#[test]
fn mutated_frames_either_reject_or_reencode_canonically() {
    let bases = [encode(&request()).unwrap(), encode(&display()).unwrap()];
    for seed in 1_u32..=1024 {
        let mut state = seed;
        let mut bytes = bases[seed as usize % 2].clone();
        for _ in 0..3 {
            state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            let index = state as usize % bytes.len();
            bytes[index] ^= (state >> 24) as u8;
        }
        if let Ok(message) = decode(&bytes) {
            assert_eq!(encode(&message).unwrap(), bytes);
        }
    }
}

#[test]
fn maximum_run_count_and_text_totals_round_trip_without_weakening_bounds() {
    let Message::Display(mut list) = display() else {
        unreachable!()
    };
    let DisplayItem::Text(run) = &mut list.items[0];
    run.text = "x".into();
    list.items = vec![list.items[0].clone(); MAX_RUNS];
    let maximum = Message::Display(list.clone());
    let bytes = encode(&maximum).unwrap();
    assert!(bytes.len() <= MAX_FRAME_BYTES);
    assert_eq!(decode(&bytes).unwrap(), maximum);
    list.items.push(list.items[0].clone());
    assert!(matches!(
        encode(&Message::Display(list)),
        Err(IpcError::InvalidDisplay)
    ));

    let Message::Display(mut list) = display() else {
        unreachable!()
    };
    let DisplayItem::Text(run) = &mut list.items[0];
    run.text = "x".repeat(MAX_INPUT_BYTES);
    list.items = vec![list.items[0].clone(); MAX_TOTAL_TEXT_BYTES / MAX_INPUT_BYTES];
    let maximum = Message::Display(list.clone());
    assert_eq!(decode(&encode(&maximum).unwrap()).unwrap(), maximum);
    let mut extra = list.items[0].clone();
    let DisplayItem::Text(run) = &mut extra;
    run.text = "x".into();
    list.items.push(extra);
    assert!(matches!(
        encode(&Message::Display(list)),
        Err(IpcError::InvalidDisplay)
    ));
}

#[test]
fn error_codes_reserved_payload_and_invalid_display_count_are_rejected() {
    let base = encode(&Message::Error {
        code: ErrorCode::Protocol,
        message: "failure".into(),
    })
    .unwrap();
    let mut bytes = base.clone();
    bytes[HEADER_BYTES..HEADER_BYTES + 2].copy_from_slice(&99_u16.to_le_bytes());
    assert!(matches!(decode(&bytes), Err(IpcError::InvalidErrorCode)));
    let mut bytes = base;
    bytes[HEADER_BYTES + 2] = 1;
    assert!(matches!(decode(&bytes), Err(IpcError::Reserved)));
    let mut bytes = header(2, 12);
    bytes.extend_from_slice(&64_u32.to_le_bytes());
    bytes.extend_from_slice(&80_u32.to_le_bytes());
    bytes.extend_from_slice(&(MAX_RUNS as u32).to_le_bytes());
    assert!(matches!(decode(&bytes), Err(IpcError::Truncated)));
}

#[test]
fn fragmented_streams_and_multiple_frames_preserve_boundaries() {
    struct Fragmented(Cursor<Vec<u8>>);
    impl Read for Fragmented {
        fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
            let count = bytes.len().min(3);
            self.0.read(&mut bytes[..count])
        }
    }
    struct ShortWriter(Vec<u8>);
    impl Write for ShortWriter {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            let count = bytes.len().min(2);
            self.0.extend_from_slice(&bytes[..count]);
            Ok(count)
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    let mut writer = ShortWriter(Vec::new());
    write_message(&mut writer, &request()).unwrap();
    write_message(&mut writer, &display()).unwrap();
    assert!(matches!(decode(&writer.0), Err(IpcError::TrailingPayload)));
    let mut reader = Fragmented(Cursor::new(writer.0));
    assert_eq!(read_message(&mut reader).unwrap(), request());
    assert_eq!(read_message(&mut reader).unwrap(), display());
    assert!(matches!(
        read_message(&mut reader),
        Err(IpcError::Truncated)
    ));
}
