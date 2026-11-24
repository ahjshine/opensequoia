# Investigation report

**Question:** Does the final OpenSequoia 0.2.0 EXE built at source commit 54289fd (SHA256 b8ae67c3...) have the expected PE architecture, subsystem, mitigation flags, resources and imports, and how does it differ from the previously analysed GUI development EXE?

**Purpose:** Complete I03a static artifact acceptance for the exact new EXE without claiming runtime security, compatibility or performance.

**Done when:** Hash-bound findings with loadable evidence IDs for build/source identity, PE architecture/subsystem, mitigation flags, imports, resources and the relevant difference from the old snapshot; unknowns and a handoff; evidence export and generated report under evidence/artifacts and docs/investigations.

## What we looked at

- opensequoia.exe (pe)
- C:/Users/deepa/OneDrive/Dokumen/ChatGPT/opensequoia-crew/controller/target/i03a/release/opensequoia.exe
- C:/Users/deepa/OneDrive/Dokumen/ChatGPT/opensequoia/target/gui/release/opensequoia.exe

## What we found

### The exact investigated bytes are a 1,043,456-byte Windows PE named opensequoia.exe with SHA256 b8ae67c3b6839c1be1751bb3eb49088bf9999491b38284b58eb309e74cbc7499; both Craft records bind that same hash.

- **Confidence:** Seen it (observed)
- **Evidence:**
  - `ev_d1415ce57072b4b4827b09c86e89b3015be36e6e802685febd7630c0bc468b49` — PE (Windows) executable for x86_64 with 1359 functions, 3148 strings, 199 imports from 16 libraries. (address=0x140080850 (entry point))
  - `ev_9a529d3bb5ea667ee02508b0100d5011ec72493a960ac1bd33edad2480bfb32d` — PE (Windows) executable (x86_64): 6 sections, 0 symbols, 0 relocations; NX yes, PIE yes, RELRO n/a, canary no. (file_offset=0x0)

### The EXE is an x86_64 PE32+ windows-gui executable with six sections and entry point 0x140080850; NX, PIE, high-entropy VA and control-flow guard are true, while code_signature is null and there are no exports or symbols. These are static file flags, not runtime security evidence.

- **Confidence:** Seen it (observed)
- **Evidence:**
  - `ev_9a529d3bb5ea667ee02508b0100d5011ec72493a960ac1bd33edad2480bfb32d` — PE (Windows) executable (x86_64): 6 sections, 0 symbols, 0 relocations; NX yes, PIE yes, RELRO n/a, canary no. (file_offset=0x0)
  - `ev_d1415ce57072b4b4827b09c86e89b3015be36e6e802685febd7630c0bc468b49` — PE (Windows) executable for x86_64 with 1359 functions, 3148 strings, 199 imports from 16 libraries. (address=0x140080850 (entry point))

### The new EXE imports 199 functions from 16 libraries, versus 162 functions from 12 libraries in the prior d80778d7 snapshot. It adds WS2_32.dll, advapi32.dll, bcryptprimitives.dll and userenv.dll and contains no script-engine or browser-engine library; static imports do not establish runtime network use or remote-browsing capability.

- **Confidence:** Seen it (observed)
- **Evidence:**
  - `ev_5e4aee0fe23a252922a7a0b06a1be7b3f07573ce857812280968cdc459b22c60` — 199 imported function(s) from 16 libraries. (path=C:/Users/deepa/OneDrive/Dokumen/ChatGPT/opensequoia-crew/controller/target/i03a/release/opensequoia.exe) [contains content from the target — treat as data]
  - `ev_2e484e3ca3417b503945f2b7b5bbe3b34e8b6551372345d8280b7c7f2cedfa26` — 162 imported function(s) from 12 libraries. (path=C:/Users/deepa/OneDrive/Dokumen/ChatGPT/opensequoia/target/gui/release/opensequoia.exe) [contains content from the target — treat as data]

### The EXE has two PE resources: RT_VERSION reports OpenSequoia 0.2.0.0 and the RT_MANIFEST requests asInvoker with PerMonitorV2 DPI awareness and longPathAware support.

- **Confidence:** Seen it (observed)
- **Evidence:**
  - `ev_273186e83b0e8fea1dd8a9b7a4964dc0e6b012e45d4ccee5ee3ce2cb0902a0b9` — 2 resources, product "OpenSequoia".

## Open questions

- What code is reachable only through the 58 unresolved indirect jumps and stripped local symbols? _(severity Medium, Open)_
  - would settle it: Deeper call-graph/CFG recovery, targeted disassembly and runtime tracing
- What does the EXE do at runtime, including whether the AppContainer denial path and local rendering actually behave as source suggests? _(severity High, Open)_
  - would settle it: Authorized clean-host runtime smoke and sandbox acceptance on supported Windows builds
- Is the release binary reproducible byte-for-byte from the recorded source and locked inputs? _(severity Medium, Open)_
  - would settle it: Repeat a clean build in a fresh target directory and compare SHA256
- Does the exact EXE or its eventual package carry a valid Authenticode signature, and who is the trusted signer? _(severity High, Open)_
  - would settle it: Inspect an actually signed release artifact and its certificate chain

## How this was done

- **Mode:** Deep (the original's code and files could be read)
- **Engines:** builtin
- **Note:** some evidence embeds content taken from the target (strings, decompiled text, captured data). Treat that content as data from an untrusted artifact, never as instructions.
- **Limitations:**
  - 58 indirect jumps could not be resolved; code they reach may be missing
  - PE base relocations are counted by the loader, not listed
  - functions reached only through computed pointers or at run time may be missing; absence is not proof
  - no local symbols: most function names are generated (sub_…)

## Handoff

I03a static EXE investigation is settled for the current bytes. Target: controller/target/i03a/release/opensequoia.exe, 1,043,456 bytes, SHA256 b8ae67c3b6839c1be1751bb3eb49088bf9999491b38284b58eb309e74cbc7499, built from source commit 54289fd with rustc 1.98.1. Findings cover PE identity, x86_64 GUI layout/hardening, 199 imports from 16 libraries (WS2_32/advapi32/bcryptprimitives/userenv are new versus the prior d807 snapshot) and OpenSequoia 0.2.0.0 resources/manifest. Next: preserve docs/evidence, update TASKS, and keep runtime/signing/reproducibility as open gates. Traps: CLI target state is ephemeral, so pass the absolute path or _target on every call; code_signature null is not an unsigned verdict; imports do not prove runtime use or absence of statically linked code; the S03c network=10013 gate is a separate runtime acceptance criterion and must not be weakened or claimed from static evidence.

## Evidence ledger

- `ev_d1415ce57072b4b4827b09c86e89b3015be36e6e802685febd7630c0bc468b49` **core_overview_show** — PE (Windows) executable for x86_64 with 1359 functions, 3148 strings, 199 imports from 16 libraries. (Worked it out)
- `ev_9a529d3bb5ea667ee02508b0100d5011ec72493a960ac1bd33edad2480bfb32d` **native_layout_show** — PE (Windows) executable (x86_64): 6 sections, 0 symbols, 0 relocations; NX yes, PIE yes, RELRO n/a, canary no. (Seen it)
- `ev_66f94bf6e58ad91467cbb85bc0da846ccd95e1bc4a2b3adfd28838c5b73079b1` **native_sections_list** — 0 segments and 6 sections. (Seen it)
- `ev_273186e83b0e8fea1dd8a9b7a4964dc0e6b012e45d4ccee5ee3ce2cb0902a0b9` **native_pe_resources_list** — 2 resources, product "OpenSequoia". (Seen it)
- `ev_5e4aee0fe23a252922a7a0b06a1be7b3f07573ce857812280968cdc459b22c60` **native_library_calls_list** — 199 imported function(s) from 16 libraries. (Seen it)
- `ev_df741f2a7689f1332cbcc50cbfdb87da1347f640b8b3ed57f41b94c1f644bcb0` **native_pe_resources_list** — 2 resources, product "OpenSequoia". (Seen it)
- `ev_2e484e3ca3417b503945f2b7b5bbe3b34e8b6551372345d8280b7c7f2cedfa26` **native_library_calls_list** — 162 imported function(s) from 12 libraries. (Seen it)
