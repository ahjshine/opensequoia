# R03 independent application wave 2 review

Reviewer: brave_baseline agent, separate from model/native/process authors and root integration. Selection: 2026-11-22. Scope: 0.2.0 local application model/controller, native controls, renderer Job/pipe boundary, main/input integration, resources and verification/package scripts. **Accepted for the bounded local development milestone. Not a full browser, sandbox, standards, accessibility or performance acceptance.**

Installed gstack/native source-review workflows from R01/R02 were reused in their documented degraded mode. No absent gstack helper receipt, outside-model review or remote-CI run is claimed. Source questions used direct code/API-documentation reads. Artifact internals remain Craft's responsibility; R03 did not parse executables or libraries.

## Findings and fixes reviewed

| ID | Reproducible defect or evidence gap | Disposition |
|---|---|---|
| R03-F1 | Invalid Go set a notice but retained rejected native EDIT text and the previous page frame through `refresh(hwnd,false)`. After Go/Enter this could show old content under a new rejected address. Source counterexample: enter an HTTPS address while Welcome is displayed. | Invalid Go now restores committed native address with `refresh(true)` while leaving model/history/frame intact. Actual app smoke submits rejected HTTPS through the same BUTTON/WM_COMMAND path and checks native EDIT plus current key unchanged, then successfully navigates to Guide. Re-read and independently executed. |
| R03-F2 | A tab pending at width 960 could become inactive, miss a width 640 reflow, then be selected while still Loading. Selection only reflowed Ready tabs; its later960 reply was accepted and never reflowed. | `accept_replies` checks the active completed viewport and emits a same-source Reflow request; submission/native calls occur outside the state borrow. The delayed-reply regression uses real engine display lists at 960/640 and model tab transitions, verifies new generation/no history change, final 640 frame and rejection of the late old token. Independently passed. |
| R03-F3 | Descendant-limit test accepted any PowerShell process-start exception as DENIED, so unrelated failures could supply false positive evidence. | Same direct ProcessStartInfo/stdio/environment/Job pipeline now has an ActiveProcessLimit=2 successful start/kill/reap positive control, followed by production limit 1 with exact `DENIED:1816` quota error. Production public launch remains limit1. Independently passed. |
| R03-F4 | New Welcome fixture no longer contained the smoke's old `rendering` marker; the author smoke log reported stage 0 content mismatch. This was a stale oracle, not proof of successful navigation. | Marker is now exact Welcome `sunlight.` versus Guide `Browse`, preserving wrong-page discrimination rather than weakening to nonempty output. Fixture hashes are retained; current real app smoke passed. Earlier author failure was read as reused evidence, not an independently run RED check. |

No additional concrete current model/controller/native-ownership/codec/Job defect was identified after these fixes. This is a scoped null result, not proof that all malformed input, OS failures or future browser integration are safe.

## Model, scheduling and cache review

Model has private nonempty tabs/histories and monotonic stable identities; navigation, reload/reflow, back/forward, tab selection/close and bounded errors were traced through their consumers. Validation preserves local filename spelling, rejects controls/bidi control characters and unsupported schemes/devices, and checks counter/history/tab preconditions before mutating model state. Closing the final development tab creates a new Welcome identity; this explicit development behavior is not full Brave last-window parity. History/profile persistence remains missing.

Controller performs file reads, child launch/waits and engine work on one background thread, outside the shared-state mutex. Pending reflows coalesce per tab, queue position remains bounded, publication checks latest key, and cancellation removes all authority/cache/queued results for the closed ID. Native acceptance additionally requires `Browser::complete` before storing a display. Navigation/reload clear previous-source frames; same-source reflow may retain the prior frame while pending. New response validation binds a successful child Display to the requested width.

Reviewed shutdown and poison/stopped-thread paths. Controller Drop marks stopping/clears state and wakes the thread; it does not block the UI joining an active renderer. That running call may finish later, but cannot publish after stop. Windows Job closure supplies process-level parent-death cleanup. This is not immediate cancellation of a currently running tab job or arbitrary filesystem operation. File/device/startup operations can have OS latency beyond the IPC exchange deadline; atomic file/volume path policy and complete end-to-end cancellation remain work before hostile browsing.

Memory is finite under current counts: at most 32 open/latest/pending/result/HTML entries, at most 128 history entries per tab and 4096 address bytes; each HTML input/cache is capped 256 KiB (up to 8 MiB of cached HTML, derived). Per-display IPC run/text limits still apply. UI cached frames and undrained controller results can coexist, and allocations can exceed their wire size. There is no aggregate display/result byte budget or eviction policy yet. Measure idle/multi-tab/peak/recovery before claiming minimal RAM or setting production budgets; count caps are not a performance comparison.

## Native UI and unsafe boundaries

UI state and Chrome ownership stay on the UI thread. Browser/controller work returns plain values; no HWND stores Rust pointers. RefCell guards end before calls that can reenter Win32, and paint uses owned Rc display snapshots. WM_COMMAND checks numeric IDs/notification class and matching owned sender HWND; accelerator IDs are handled separately. This validates program routing, not an authenticated cross-process security channel. Native edit length/UTF-16, label bounds, tab mapping, focus/accelerator paths, font/control lifetime, partial-creation cleanup, clipping/paint pairing, DPI/scroll arithmetic and parent-first teardown were read.

Actual smoke exercises the native control path and observes asynchronous child content, native address restoration, navigation/history/tabs/last-tab reset and paint. Its internal BM_CLICK operations do not establish physical keyboard, UI Automation, Narrator or complete accessibility parity. Native-control defaults help but page accessibility, IME/selection/high contrast, full typography/shaping and smooth high-resolution scrolling remain unresolved. Actual manual keyboard/scroll verification belongs to root's authorized CUA work; this reviewer did not operate user browsers or other windows.

## Windows process and pipe review

Read actual Win32 source and captured Microsoft CreateProcess/UpdateProcThreadAttribute/Job/cancellation documentation. JOB_LIST assigns the configured unnamed Job during creation; Windows 10+/Server 2016+ support is documented but only this Windows 11 x64 host ran. HANDLE_LIST admits the three child stdio ends. Parent ends and the Job are noninheritable; no parent process/Job/thread handle is transmitted. The Job has kill-on-last-close, one active process and 256 MiB process-private-memory development limit.

Attribute sizing is checked/capped; aligned initialized backing and handle arrays remain live through process creation and attribute-list deletion. OwnedHandle/File ownership transfers once; native strings are bounded terminated UTF-16, executable path is explicit/absolute, arguments are quoted as data, and no PATH/shell lookup occurs in the production launcher. Environment is limited to SystemRoot/WINDIR. Owned process handle/PID/Job identity is checked. Job-wide termination, exit observation and CancelSynchronousIo target the owned child/thread, with bounded completion/failure handling.

Real tests verify membership/job-close termination; quota denial with successful-start control; unrelated handle exclusion with included-marker control; synchronous read cancellation with writer still open and exact 995 result; parent death with child observation handle opened before parent termination. The development helper includes the real platform module and compiles outside Cargo only for these probes; it is not packaged application implementation.

Jobs, handle lists, environment pruning and CFG do **not** establish AppContainer/restricted-token least privilege, filesystem/network denial, site/origin isolation or authenticated capability IPC. A renderer with arbitrary code execution retains ambient user privileges; before remote content, the remaining sandbox/origin/TLS/permission gates are mandatory. Cancellation API success marks requests and does not itself prove completion; the code waits/checks outcomes and retains explicit failure limits.

## Independent verification actually executed

Ran `scripts/verify.ps1 -EvidencePrefix review3` after source freeze, serially without overlapping duplicate helper builds or root CUA. **Exit0**, `DEVELOPMENT_GATES_OK`. Environment: Windows 11 x64, Rust/Cargo 1.98.1, MSVC/Windows SDK and current CFG compiler configuration, local crates 0.2.0. Evidence: `evidence/verification/review3-*.log` and `review3-verification.json`.

| Gate | Observed outcome |
|---|---|
| Formatting / Clippy | current workspace format passed; all targets Clippy with `-D warnings` passed |
| Workspace behavior | **35 app + 9 engine + 10 IPC = 54 passed**, 0 failed/ignored; counts read from actual logs |
| Release build | current0.2.0 optimized executable built successfully |
| Headless inspection | actual bundled fixture nodes 49, width 960, height 552, text runs 169; one explicit unsupported-feature diagnostic |
| Child lifecycle smoke | actual healthy/malformed/restart; 3 direct children spawned/reaped |
| Native paint smoke | actual child-rendered paint 1, visible text runs 105, successful teardown |
| Native application smoke | actual navigation/back/forward/tabs/close-reset/native-controls/child content passed; 11 paints; rejected EDIT restoration included |

`review3-verification.json` hashes the consumed crate/config/fixture inputs. Independently compared those hashes with current files after execution: no mismatch. `review3-extra-source-hashes.json` also captures verification/package/workflow/README and test-helper source. Re-run if any consumed source changes. Existing older 30-test evidence is historical, not the current total.

No WPT/Test262/Wasm conformance, production sandbox/token/TLS/origin/extension/download tests, full accessibility or comparative runtime performance passed by inference from these gates. Source dependency graph remains the reviewed Rust graph; no new registry runtime was introduced for navigation/Job/UI. Artifact metadata/native completeness, final ZIP correspondence, signing and installation/update remain root/Craft/package gates. Remote CI configuration was inspected, not remotely executed.

## Handoff

Root may accept the local 0.2 application/navigation/renderer ownership leaves and integrate the returned review before committing. Update root architecture/parity/checkpoint records from the historical in-process/probe description to the actual background-controller/child/native-control behavior, preserving all full-browser gaps. Next: AppContainer/least privilege and origin/capability/TLS boundaries; versioned conformance/DOM/CSS work; bounded cancelable tab/file lifecycle; aggregate cache/peak memory measurements; actual keyboard/page accessibility; final Craft/source/package fingerprint reconciliation. No mission constraint or failing required gate was deleted to reach this milestone.
