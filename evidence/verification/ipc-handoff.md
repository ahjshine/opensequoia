# S02a bounded renderer IPC handoff

Owner `/root/rust_feasibility`; run date 2026-11-22. State **Under review**: package self-checks pass; parent independent review and child-process runtime remain required. This is a development transport, not production secure IPC or sandbox completion.

## Files / interfaces

Owned files: `crates/ipc/src/lib.rs`, `crates/ipc/src/codec.rs`, `crates/ipc/tests/framing.rs`, `evidence/verification/ipc-*.txt`, this handoff and `ipc-source-hashes.json`. Root added manifest/workspace/path dependency and owns integration.

Public crate `opensequoia_ipc`:

```rust
enum Message {
    Render { html: String, width: u32 },
    Display(opensequoia_engine::DisplayList),
    Error { code: ErrorCode, message: String },
}
enum ErrorCode { Parse, Layout, Protocol }
fn encode(message: &Message) -> Result<Vec<u8>, IpcError>;
fn decode(frame: &[u8]) -> Result<Message, IpcError>;
fn read_message<R: std::io::Read + ?Sized>(reader: &mut R) -> Result<Message, IpcError>;
fn write_message<W: std::io::Write + ?Sized>(writer: &mut W, message: &Message) -> Result<(), IpcError>;
```

`write_message` does not flush. Caller must flush buffered pipe output. On any protocol/IO error discard the connection; no resynchronization or application retry. `read_message` consumes exactly one frame and leaves following frames unread. `decode` accepts exactly one whole frame and rejects extra bytes. Read deadlines/cancellation/process kill and reap are caller responsibilities.

## Version 1 wire contract

All fields little endian, exact-length UTF-8 strings, no native Rust object layout or implicit padding. Header 16 bytes: magic `SQIP`[4], version:u16=1, tag:u8, reserved:u8=0, payload_bytes:u32, reserved:u32=0. Whole frame including header <=4 MiB. Unknown magic/version/tag/reserved values fail closed before payload allocation/read. Type-specific payload caps are stricter than global cap.

| Tag | Payload |
|---|---|
| 1 Render | width:u32, html_bytes:u32, UTF-8 html[html_bytes] |
| 2 Display | width:u32, height:u32, count:u32, then count text records |
| Text record | x:i32,y:i32,width:u32,height:u32,font_size:u32,bold:u8(0/1),r:u8,g:u8,b:u8,text_bytes:u32,text[text_bytes] (28-byte fixed header) |
| 3 Error | code:u16 (1 Parse/2 Layout/3 Protocol), reserved:u16=0, message_bytes:u32, UTF-8 message[message_bytes] |

Every payload must be fully consumed. Render HTML <=262144 UTF-8 bytes and width 64..8192. Display width 64..8192, height 1..1000000, <=65536 runs, each nonempty UTF-8 text <=262144 bytes, total text <=1048576 bytes, font 1..256, positive run dimensions, nonnegative coordinates and checked rectangle ends within display bounds. Error message <=1024 UTF-8 bytes. These are resource/protocol limits, not verified typographic bounds; glyph shaping/actual drawing extent remains platform work.

## Acceptance / evidence

| Procedure actually run | Result | Evidence |
|---|---|---|
| `cargo test -p opensequoia-ipc --test framing round_trip_all_message_types_and_streamed_frames` against initial error stub | Expected real RED, exit101; round-trip test failed `BadMagic` | `ipc-red.txt` |
| `cargo test -p opensequoia-ipc --locked` after full implementation/final added checks | Exit0, 10 passed/0failed/0ignored; final suite0.24s | `ipc-green.txt` |
| `cargo clippy -p opensequoia-ipc --all-targets --locked -- -D warnings` | Exit0, no warnings | `ipc-clippy.txt` |
| `cargo fmt -p opensequoia-ipc -- --check` | Exit0 (silent success) | `ipc-format.txt` |

Tests exercise real encode/decode and standard IO streaming: all message types, fragmented reads/short writes/multiple frames, every truncated prefix of Unicode request/display frames, non-UTF8, outer/payload trailing bytes, magic/version/tag/header and error reserved fields, invalid code/boolean/viewport/display geometry/fonts/text/count, inclusive max HTML/errors/count/total text and one-over rejection. 1024 deterministic three-byte frame mutations either reject or reencode identically; this is bounded generated testing, not a coverage-guided fuzz campaign.

Oversize-before-allocation evidence is source control flow (`header` before payload Vec) plus a reader positive control that fails if payload is read; oversized global/type declarations and unknown tags trigger no payload read. No allocator instrumentation, measured peak memory, runtime security proof or formal verification is claimed.

Environment: same pinned Rust1.98.1 x86_64 Windows MSVC workspace as S01. No new third-party codec/runtime dependencies; IPC encoding uses standard library and engine display value types. Crate forbids unsafe. Root retains locked transitive engine dependency audit.

## Limits / next tasks

No peer identity, nonce/handshake, origin/document IDs, capabilities, handle transfer, sandbox, ACLs, transport authentication or site isolation exists in v1. The caller must reject responses inconsistent with its request/lifecycle. Windows child stdio is a local development lifecycle probe. Malicious valid frames can still exercise bounded CPU/memory work; whole-process quotas/deadlines and concurrency bounds remain essential.

Next: parent independent source review/reverification; Windows agent integrates `--renderer-worker` and timeout/kill/reap/restart smoke against the real parser/layout. Maintain local-only restriction until AppContainer/least-privilege/site isolation and authenticated capability-bearing production IPC pass their own gates. No engine fallback or unavailable integration was added.
