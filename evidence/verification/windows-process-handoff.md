# S02b local renderer lifecycle handoff

Owner windows_security, 2026-11-22. Files: crates/app/src/worker.rs plus main/input mode integration; root owns IPC crate/dependency manifest and integration. No commits or Craft session changes.

Behavior: own executable --renderer-worker receives one bounded Render frame, runs Rust Document parse/layout, writes/flushes Display or fatal Error, and exits. --process-smoke uses only the bundled local fixture and validates three actual children: healthy response/exit, bad-magic Protocol error/nonzero exit, then fresh-child recovery. All three direct children reaped before success verdict. No GUI/sandbox/site identity/peer authentication/capability/network/profile functionality is claimed.

Supervisor uses concurrent pipe I/O, 5s reply/exit deadline, 1s termination/reap deadline and 1s I/O-finished guard before join. Child environment cleared except Windows loader context. Failed cleanup is an error, not a success. Current worker spawns no descendants: direct-child cleanup does not prove whole-tree cleanup. Job ownership, child-spawn denial/handle policy/cancelable I/O remain required before exposing future hostile workloads. A still-blocked I/O thread fails/detaches; this is a documented failure state, not verified cleanup.

Actual latest verification on Rust 1.98.1, Windows 11 build 26200, x86_64 MSVC:

- cargo test -p opensequoia --locked: 11 passed, 0 failed, 0 ignored after Windows CFG rebuild, including actual controlled sleep-child timeout/kill/reap plus real local junction, worker parse/layout/protocol, OS ASCII font metrics and unfinished-I/O helper branch. Final helper refinement separately passed its focused test; controlled fixtures release/join even on timeout so a regression does not hang the test. This is not pipe cancellation proof.
- cargo clippy -p opensequoia --all-targets --locked -- -D warnings: exit 0, no warnings.
- cargo run -p opensequoia --locked -- --process-smoke: exit 0, PROCESS_SMOKE_OK healthy=1 malformed_rejected=1 restarted=1 spawned=3 reaped=3.
- Post-ASCII-fix native --smoke-test: exit 0, paints 1 / drawn 113. No CUA after user stopped it; scroll/Escape runtime interaction not passed.

Logs: windows-worker-tests.log, windows-worker-join-test.log, windows-worker-clippy.log, windows-process-smoke.log, windows-smoke.log. Native/process smoke logs predate the CFG rebuild and final test-only refinement; root must independently rerun final integrated runtime checks. ASCII metric red/green evidence: windows-font-red.log (245px > 207px allocation), windows-font-green.log (same regression passed 32 size/bold/DPI combinations after fixed Consolas cells). Unicode/font shaping and standard CSS metrics remain unsupported.

Review status: R02 review requested; reviewer identified direct-child/unbounded-join caveat, now bounded finished polling added and assumption retained. Root must independently run final whole-workspace/release checks and artifact review; no full browser security/parity/compatibility/performance claim. Next: broker-owned document/site/capability contracts, authenticated IPC authority, actual restricted child/job/token/access-denial tests, then integrate UI renderer lifecycle before remote navigation.
