# Investigation report

**Question:** Does the final OpenSequoia 0.2.0 GUI development executable use PE subsystem 2 with the expected architecture, hardening, branding and runtime imports, and does its deterministic package contain those exact bytes and required license files?

**Purpose:** Finish I02a GUI package acceptance and correct the console draft without claiming browser release or runtime security/performance.

**Done when:** Hash-linked GUI subsystem/AMD64/CFG/version/import findings and complete ZIP membership with intact evidence IDs, documented limitations and saved handoff/export/report.

## What we looked at

- 158.1.100.58 (directory)
- BraveBrowserStandaloneSetup-1.100.58.exe (pe)
- app2-opensequoia.exe (pe)
- brave.exe (pe)
- chrome.dll (pe)
- craft-opensequoia-final.exe (pe)
- craft-opensequoia-s02.exe (pe)
- opensequoia-0.1.0-win-x64.zip (zip)
- opensequoia-0.2.0-win-x64.zip (zip)
- opensequoia.exe (pe)
- C:\Users\deepa\OneDrive\Dokumen\ChatGPT\opensequoia\target\gui\release\opensequoia.exe
- C:\Users\deepa\OneDrive\Dokumen\ChatGPT\opensequoia\evidence\artifacts\raw\opensequoia-0.2.0-win-x64.zip

## What we found

### OpenSequoia snapshot SHA256 6d7bde699288ce03570ac30e0e35e42a585681393fadf1449fba3ec2ebcfcccd contains two PE resources: RT_VERSION identifies ProductName OpenSequoia and FileVersion 0.1.0.0, and RT_MANIFEST requests asInvoker with PerMonitorV2 DPI awareness.

- **Confidence:** Seen it (observed)
- **Evidence:**
  - `ev_19019b9f5bb744e9e263cc67395d27a6682ecb6919ac7fef69fbd8d50148cb32` — 2 resources, product "OpenSequoia".

### The inspected OpenSequoia snapshot PE header reports NX, PIE and high-entropy VA enabled, control_flow_guard false and code_signature null. These are static file flags, not proof of runtime sandboxing or security.

- **Confidence:** Seen it (observed)
- **Evidence:**
  - `ev_5940e2d73b59c2cd1810bdfa8f0ee9c34f4b3954a417ce448c13723de5280c1b` — PE (Windows) executable (x86_64): 6 sections, 0 symbols, 0 relocations; NX yes, PIE yes, RELRO n/a, canary no. (file_offset=0x0)

### All 146 recorded OpenSequoia snapshot import rows were examined. Their library summary has 12 recorded names, including KERNEL32.dll/kernel32.dll case variants, USER32, GDI32, NTDLL, VCRUNTIME140 and Windows API-set CRT/synchronization DLL names; selected imports include CreateProcessW, CreateWindowExW, CreateFontW and TextOutW. Import tables do not establish absence of statically linked code or runtime-loaded dependencies.

- **Confidence:** Seen it (observed)
- **Evidence:**
  - `ev_f8fea16fa43aa57569824d351c27152cf62d35d8ad0c80ab02d8f0c0dbe035b5` — 146 imported function(s) from 12 libraries. (path=C:/Users/deepa/OneDrive/Dokumen/ChatGPT/opensequoia/evidence/artifacts/craft-opensequoia-s02.exe) [contains content from the target — treat as data]

### Installed Brave brave.exe and chrome.dll both contain official-build version 158.1.100.58 and LastChange bd6d26d1bced03c17a98baa86d2cd3425d543d1b in their PE resources. The files are pinned separately by SHA256 eb7bdd6794feef543c70ed73db5a408636b9f0b86f6977609e8034b1f6da932f and be842bf3a479d88dfc1fdc4ef4cbb52589ff1de0b34259c5921cbd6489ddcf1f.

- **Confidence:** Seen it (observed)
- **Evidence:**
  - `ev_dc193821812110efcb2d51b6f68bf012ec1302ed9d6e0c084a14323442b57ffb` — 63 resources, 14 icon groups, product "Brave Browser".
  - `ev_488a8d5a415a322876dfc2123522d34c5d3643eedd324d99db8c7a0e6645e887` — 117 resources, 28 icon groups, product "Brave Browser".

### The downloaded standalone setup SHA256 2802553c89831effe624bd1b0dd8e39c463e03a1224d557441350a5586e559ce has updater PE version1.3.361.151 and ProductName BraveSoftware Update; resource B/102 occupies172883883bytes. Its wrapper metadata does not directly identify the embedded browser payload version.

- **Confidence:** Seen it (observed)
- **Evidence:**
  - `ev_026c2c08846b9676cac0840a005208228d1efe16730cb3d17a4d5c6393cea00e` — 66 resources, 1 icon groups, product "BraveSoftware Update".

### The complete selected Brave version-folder inventory has377files and1146695025hashed file bytes. Largest contributors are Installer/chrome.7z569442764bytes and chrome.dll338606160bytes; the inventory covers installed files, not filesystem allocated clusters or profile data.

- **Confidence:** Worked it out (derived)
- **Evidence:**
  - `ev_99e11378acc62a330777ece005827fef264a7769cc3896b420f05870ceebc8f5` — 158.1.100.58 (directory) holds 377 files in 455 entries; 0 nested container(s). (path=C:/Program Files/BraveSoftware/Brave-Browser/Application/158.1.100.58)

### The installed baseline contains BraveVpnWireguardService/brave_vpn_wireguard_service.exe, tunnel.dll, wireguard.dll and brave_vpn_helper.exe with total24467776file bytes. These packaged VPN components support the authorized exclusion inventory; runtime service use and their contribution to startup/RAM were not measured here.

- **Confidence:** Worked it out (derived)
- **Evidence:**
  - `ev_99e11378acc62a330777ece005827fef264a7769cc3896b420f05870ceebc8f5` — 158.1.100.58 (directory) holds 377 files in 455 entries; 0 nested container(s). (path=C:/Program Files/BraveSoftware/Brave-Browser/Application/158.1.100.58)

### The final OpenSequoia executable SHA256930a4415a37137ad33ce77e4c82c134aa1ae0be5dcae14bc8449104ffb7042b8 advertises Control Flow Guard, NX, PIE and high-entropy VA. Its version/manifest resources still identify OpenSequoia0.1.0.0; code_signature is null. This is static metadata and does not establish production sandboxing or authentic signed updates.

- **Confidence:** Seen it (observed)
- **Evidence:**
  - `ev_5c812b8afc08c37a8703187d37d55c51dc6ec0d6532cc81a074ac6d89b5ff51a` — PE (Windows) executable (x86_64): 6 sections, 0 symbols, 0 relocations; NX yes, PIE yes, RELRO n/a, canary no. (file_offset=0x0)
  - `ev_15a6b690669ec0bd4b2b9cb3e76f6b6be92cda0177889ec448ab11d12c37518b` — 2 resources, product "OpenSequoia".

### Successive observed OpenSequoia build hashes differ, and the recorded PE control_flow_guard flag changes from false in6d7bde699288ce03570ac30e0e35e42a585681393fadf1449fba3ec2ebcfcccd to true in930a4415a37137ad33ce77e4c82c134aa1ae0be5dcae14bc8449104ffb7042b8. This confirms the final artifact flag; it is not a controlled performance comparison.

- **Confidence:** Worked it out (derived)
- **Evidence:**
  - `ev_5940e2d73b59c2cd1810bdfa8f0ee9c34f4b3954a417ce448c13723de5280c1b` — PE (Windows) executable (x86_64): 6 sections, 0 symbols, 0 relocations; NX yes, PIE yes, RELRO n/a, canary no. (file_offset=0x0)
  - `ev_5c812b8afc08c37a8703187d37d55c51dc6ec0d6532cc81a074ac6d89b5ff51a` — PE (Windows) executable (x86_64): 6 sections, 0 symbols, 0 relocations; NX yes, PIE yes, RELRO n/a, canary no. (file_offset=0x0)

### The first32observed bytes of updater resource B/102 are5d0000800078678a0a0000000000002abfeb5c8f2c0c6fbf98b98638c602eb80. The prefix is consistent with LZMA-Alone framing, an8MiB dictionary and declared176842616unpackedbytes; decompression validity and embedded browser file identities were not established.

- **Confidence:** Best guess (inferred)
- **Evidence:**
  - `ev_f6089e1ab3d105eb0acc8bf494da10921f662fc0568f28ebdc129064dcb78696` — 32 bytes at 0x426824. (address=0x426824)
  - `ev_026c2c08846b9676cac0840a005208228d1efe16730cb3d17a4d5c6393cea00e` — 66 resources, 1 icon groups, product "BraveSoftware Update".

### The final development ZIP SHA256e1f6b22bb9f84113fc3f873e57a682d7b83cceb09936d083c172fc10b68183bf inventory is complete: opensequoia.exe908800bytes with SHA256930a4415a37137ad33ce77e4c82c134aa1ae0be5dcae14bc8449104ffb7042b8, LICENSE17099bytes, README.md2585bytes and THIRD_PARTY_NOTICES.txt322482bytes. The four members total1250966unpacked file bytes; no ZIP contradictions were reported.

- **Confidence:** Seen it (observed)
- **Evidence:**
  - `ev_ec8eed8efe42aa4f623e196215e20703eff68bf5c2ac5f9693bf7c9285e3f8b0` — opensequoia-0.1.0-win-x64.zip (zip) holds 4 files in 4 entries; 0 nested container(s). (path=C:/Users/deepa/OneDrive/Dokumen/ChatGPT/opensequoia/evidence/artifacts/raw/opensequoia-0.1.0-win-x64.zip)

### Craft native-layout reports code_signature:null for the inspected Brave executable. This field is not an Authenticode signature-validation verdict; separate Windows signature verification is required to establish signer/trust status.

- **Confidence:** Seen it (observed)
- **Evidence:**
  - `ev_428782241cfb9a0eb59ca948ff23d0b413dab1a3e6e3e0bc6a0bc8cfdf0bc50d` — PE (Windows) executable (x86_64): 11 sections, 0 symbols, 0 relocations; NX yes, PIE yes, RELRO n/a, canary no. (file_offset=0x0)

### The final OpenSequoia executable records146imports from12library names including VCRUNTIME140.dll and Windows API-set CRT names. The development ZIP contains only the executable and three documentation/license files; clean-machine runtime prerequisite resolution is not established by this inventory, and no statically-linked native-code absence claim follows.

- **Confidence:** Worked it out (derived)
- **Evidence:**
  - `ev_7bde15503add490eaab3b678e0ba974cb9c876725ef2b930f407dcf6733d9b5c` — 146 imported function(s) from 12 libraries. (path=C:/Users/deepa/OneDrive/Dokumen/ChatGPT/opensequoia/evidence/artifacts/craft-opensequoia-final.exe) [contains content from the target — treat as data]
  - `ev_ec8eed8efe42aa4f623e196215e20703eff68bf5c2ac5f9693bf7c9285e3f8b0` — opensequoia-0.1.0-win-x64.zip (zip) holds 4 files in 4 entries; 0 nested container(s). (path=C:/Users/deepa/OneDrive/Dokumen/ChatGPT/opensequoia/evidence/artifacts/raw/opensequoia-0.1.0-win-x64.zip)

### The complete0.2snapshot import table has162normal import rows from12recorded library names, including Windows services and VCRUNTIME140/API-set CRT dependencies. It records CreateJobObjectW, SetInformationJobObject, TerminateJobObject and Initialize/Update/DeleteProcThreadAttribute APIs. Import presence does not prove Job limits, execution order, privilege or site/origin isolation.

- **Confidence:** Seen it (observed)
- **Evidence:**
  - `ev_751a26dab5578a14416d3b7ad83c50792deaae9b46901ef57c64836d041ee258` — 162 imported function(s) from 12 libraries. (path=C:/Users/deepa/OneDrive/Dokumen/ChatGPT/opensequoia/evidence/artifacts/app2-opensequoia.exe) [contains content from the target — treat as data]

### Comparing complete0.1and0.2import tables after casefolding DLL names and deduplicating identical symbol/kind keys yields29added and13removed keys; recorded library-name set stays the same. Added keys include Job/startup-attribute/pipe APIs and native UI focus/text/accelerator/clipping APIs. This is linkage evidence, not proof of UI behavior or feature compatibility.

- **Confidence:** Worked it out (derived)
- **Evidence:**
  - `ev_751a26dab5578a14416d3b7ad83c50792deaae9b46901ef57c64836d041ee258` — 162 imported function(s) from 12 libraries. (path=C:/Users/deepa/OneDrive/Dokumen/ChatGPT/opensequoia/evidence/artifacts/app2-opensequoia.exe) [contains content from the target — treat as data]
  - `ev_7bde15503add490eaab3b678e0ba974cb9c876725ef2b930f407dcf6733d9b5c` — 146 imported function(s) from 12 libraries. (path=C:/Users/deepa/OneDrive/Dokumen/ChatGPT/opensequoia/evidence/artifacts/craft-opensequoia-final.exe) [contains content from the target — treat as data]

### The frozen OpenSequoia0.2snapshot SHA256d50b7780c3dcfcce1906e946a103c2a0a765689805b2f15b6db4ad4208160e8f has two complete PE resources: RT_VERSION names OpenSequoia0.2.0.0 and RT_MANIFEST requests asInvoker, PerMonitorV2 and long-path awareness. These resource declarations do not prove native UI behavior or production sandboxing.

- **Confidence:** Seen it (observed)
- **Evidence:**
  - `ev_4a490d4b39d95ed5c836302030c04e9b2505374c45a2364f072ab5e93cbd1a02` — 2 resources, product "OpenSequoia".

### Craft-adapted objdump header output for the frozen0.2snapshot reports pei-x86-64, PE32+magic020b, DllCharacteristics0xc160 with HIGH_ENTROPY_VA/DYNAMIC_BASE/NX_COMPAT/GUARD_CF, and Windows CUI subsystem3. The captured first180lines include the header; this raw static tool output does not prove OS enforcement, GUI behavior or production sandboxing.

- **Confidence:** Seen it (observed)
- **Evidence:**
  - `ev_9970fd550e9f4ef8708a84d22d8a30880bddc14db7c8a71bc7edcbc0b96b95ea` — objdump private_headers: 22892 lines of output.
  - `ev_4a490d4b39d95ed5c836302030c04e9b2505374c45a2364f072ab5e93cbd1a02` — 2 resources, product "OpenSequoia".

### The final GUI executable SHA256 d80778d7dbaa7f3a520b33e88ab3cb8346cce1c8c31749cba19653ede0511f3e is PE32+ AMD64 (pei-x86-64) and its first250 header-adapter lines report Windows GUI subsystem2 with HIGH_ENTROPY_VA, DYNAMIC_BASE, NX_COMPAT and GUARD_CF. Header flags are static mitigation metadata, not runtime sandbox/security proof.

- **Confidence:** Seen it (observed)
- **Evidence:**
  - `ev_a5ef3c46551c09321e8f1df477403bdfc68782d330ee8a107721730a8f3b1e3b` — objdump private_headers: 22890 lines of output.

### The final GUI executable contains exactly two enumerated PE resources: OpenSequoia RT_VERSION FileVersion0.2.0.0/ProductVersion0.2.0 and RT_MANIFEST0.2.0.0 with asInvoker, uiAccess false, PerMonitorV2 and longPathAware true. No icon group is present in the enumerated resource tree.

- **Confidence:** Seen it (observed)
- **Evidence:**
  - `ev_7c864322896125c053adf7e03d6f2f6488bcb36bc41dc676c066a3e375509527` — 2 resources, product "OpenSequoia".

### Complete GUI and console-draft import inventories have the same162 semantic imported functions from12 recorded library names (including a KERNEL32 case variant), with some import-slot order changes and no added/removed functions or DLL names. Windows/CRT imports including VCRUNTIME140 are observed; this does not prove static native code absence or clean-machine runtime availability.

- **Confidence:** Worked it out (derived)
- **Evidence:**
  - `ev_f97f3c572503836ee601339717f5779c4f277218b6637baef179d3d4a441212b` — 162 imported function(s) from 12 libraries. (path=C:\Users\deepa\OneDrive\Dokumen\ChatGPT\opensequoia\target\gui\release\opensequoia.exe) [contains content from the target — treat as data]
  - `ev_751a26dab5578a14416d3b7ad83c50792deaae9b46901ef57c64836d041ee258` — 162 imported function(s) from 12 libraries. (path=C:/Users/deepa/OneDrive/Dokumen/ChatGPT/opensequoia/evidence/artifacts/app2-opensequoia.exe) [contains content from the target — treat as data]

### The final GUI development ZIP SHA256743c6594a6e72f0dffdcfbf07537b4900659da35d018933eef39df21d871ff05 has a complete four-member inventory: opensequoia.exe946176bytes with verified GUI SHA d80778d7dbaa7f3a520b33e88ab3cb8346cce1c8c31749cba19653ede0511f3e, LICENSE17099bytes, README.md3371bytes and THIRD_PARTY_NOTICES.txt322482bytes. Total hashed member bytes1289128; no ZIP contradictions reported.

- **Confidence:** Seen it (observed)
- **Evidence:**
  - `ev_5da17a302b1ea9c68081ad590c783575f88bc1474663a074758898275d34f2bb` — opensequoia-0.2.0-win-x64.zip (zip) holds 4 files in 4 entries; 0 nested container(s). (path=C:\Users\deepa\OneDrive\Dokumen\ChatGPT\opensequoia\evidence\artifacts\raw\opensequoia-0.2.0-win-x64.zip)

## Open questions

- What exact browser payload files and source/build revision are embedded in standalone updater resource B/102? _(severity High, Open)_
  - would settle it: Supported static decompression/extraction of the observed container with strict bounds, then hash/version comparison to selected installed files and release source; do not run installer
  - would settle it: Inspect pinned upstream packaging source to establish exact wrapper/payload build relationship
- Does the unsigned0.2GUI development payload start correctly on a supported clean Windows machine without an existing VC runtime, and can it meet signed installer/update/recovery requirements? _(severity High, Open)_
  - would settle it: Run installation/startup on a disposable supported Windows10/11 x64 machine with pristine runtime prerequisites
  - would settle it: Verify signing/trusted update/recovery/install/uninstall behavior when production packaging is implemented
- Does the locked OpenSequoia release contain unexpected statically linked native implementation or runtime-loaded dependencies beyond the recorded Windows/CRT imports? _(severity High, Open)_
  - would settle it: Tie locked source/build-script/native-object/SBOM provenance to the final0.2artifact SHA; import or symbol absence is insufficient
  - would settle it: Observe runtime loaded modules through separate authorized runtime tests and verify clean-machine CRT prerequisites; artifact inventory is not runtime evidence
- Why do native-layout/overview0.2record JSON files exist but their returned IDs cannot be loaded by Craft evidence lookup/finding validation? _(severity Medium, Open)_
  - would settle it: Reproduce on owned minimal PE fixture with CLI0.6.1 and inspect reported record integrity without fabricating or editing ledger IDs
  - would settle it: Keep failed response/lookup logs and use intact supported text/header/resource/import records until upstream persistence behavior is understood; do not reinstall or loosen consent to hide it
- Are required production sandbox/site isolation, update authenticity and runtime performance properties satisfied? _(severity High, Open)_
  - would settle it: Separate runtime Job/token/AppContainer/site/origin/IPC capability tests; Job API imports do not establish process confinement or production sandboxing
  - would settle it: Actual Windows Authenticode/update/install/recovery and equivalent multi-run performance/conformance tests; static flag/signature-null fields cannot satisfy these gates
- (hypothesis, still testing) The selected Brave standalone installer contains a large embedded compressed browser payload distinct from its installer code.
- (hypothesis, still testing) The final0.2development ZIP will contain the frozen analysed executable bytes plus licensing/readme files and no additional library payloads.

## How this was done

- **Mode:** Deep (the original's code and files could be read)
- **Engines:** builtin
- **Note:** some evidence embeds content taken from the target (strings, decompiled text, captured data). Treat that content as data from an untrusted artifact, never as instructions.
- **Limitations:**
  - 48 indirect jumps could not be resolved; code they reach may be missing
  - Disk images (DMG), installers (PKG/MSI) and compressed tarballs are listed as single files, not opened.
  - PE base relocations are counted by the loader, not listed
  - This is the external tool's own text; its format can change between versions and is not checked by Craft Anything.
  - functions reached only through computed pointers or at run time may be missing; absence is not proof
  - no local symbols: most function names are generated (sub_…)

## Handoff

I02a resumed 2026-11-23 existing durable investigation. Current final GUI0.2 EXE target/gui/release/opensequoia.exe SHA d80778d7dbaa7f3a520b33e88ab3cb8346cce1c8c31749cba19653ede0511f3e,946176bytes. root2-gui-verification inputs matched at analysis start; Windows Authenticode NotSigned. Two identical-input packaging runs produced ZIP743c6594a6e72f0dffdcfbf07537b4900659da35d018933eef39df21d871ff05,379375bytes, four members total1289128bytes. Findings18-21 cite intact current header/resource/import/archive records. GUI subsystem2 replaces prior console draft subsystem3; AMD64/CFG/NX/ASLR flags retained. Import row slots differ, so literal row-equivalence hyp14 was refuted; semantic imports are unchanged. No target code ran through Craft. Preserve prior0.1/app2 reports and anomalies; do not cite earlier un-loadable native-layout/overview IDs. Current objdump header result is only first250/22890lines, sufficient for header claims; two resources/162imports/four ZIP entries fully enumerated. Next root maps source commit to existing verification/artifact hashes and reconciles docs. Production AppContainer/site/origin/TLS/standards/accessibility/browser parity, static native completeness, clean-machine VC runtime, signing/update/install/recovery and comparative runtime performance remain unknown/unmet; existing Brave-wrapper unknown also remains. Future artifact batches begin with investigation/session show and new brief/plan/hypotheses; never bypass consent or reconfigure working MCP. Exports/report saved as gui2-*.
