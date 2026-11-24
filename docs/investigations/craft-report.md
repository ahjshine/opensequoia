# Investigation report

**Question:** What PE structure, imports, resources and package composition are observed in the pinned Brave Windows artifacts and the built OpenSequoia Rust development executable, and what remains unknown about native components and feature exclusions?

**Purpose:** Establish evidence for source-to-artifact baseline, Rust implementation boundary, branding, footprint and architecture; complement source review and runtime tests.

**Done when:** Version/hash-linked static findings with supporting evidence IDs, labelled confidence, important unknowns, plan states and a resumable handoff; export a report without full browser or performance claims.

## What we looked at

- 158.1.100.58 (directory)
- BraveBrowserStandaloneSetup-1.100.58.exe (pe)
- brave.exe (pe)
- chrome.dll (pe)
- craft-opensequoia-final.exe (pe)
- craft-opensequoia-s02.exe (pe)
- opensequoia-0.1.0-win-x64.zip (zip)
- C:/Users/deepa/OneDrive/Dokumen/ChatGPT/opensequoia/target/release/opensequoia.exe
- C:/Program Files/BraveSoftware/Brave-Browser/Application/brave.exe
- C:/Program Files/BraveSoftware/Brave-Browser/Application/158.1.100.58/chrome.dll
- C:/Users/deepa/OneDrive/Dokumen/ChatGPT/opensequoia/evidence/artifacts/raw/BraveBrowserStandaloneSetup-1.100.58.exe

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

## Open questions

- What exact browser payload files and source/build revision are embedded in standalone updater resource B/102? _(severity High, Open)_
  - would settle it: Supported static decompression/extraction of the observed container with strict bounds, then hash/version comparison to selected installed files and release source; do not run installer
  - would settle it: Inspect pinned upstream packaging source to establish exact wrapper/payload build relationship
- Does the locked OpenSequoia release contain unexpected statically linked native implementation or runtime-loaded dependencies beyond the recorded Windows/CRT imports? _(severity High, Open)_
  - would settle it: Reproducible source/lockfile/build-script/native-object and SBOM provenance audit tied to final release hash
  - would settle it: Observe runtime loaded modules under a separate explicitly authorized runtime test; import-table or missing-string absence alone is insufficient
- Are required production sandbox/site isolation, update authenticity and runtime performance properties satisfied? _(severity High, Open)_
  - would settle it: Runtime process/token/mitigation/IPC attack and site/origin tests with equivalent retained functionality
  - would settle it: Actual signed update/install/recovery verification and reproducible multi-run performance baselines; static flags do not prove these
- (hypothesis, still testing) The selected Brave standalone installer contains a large embedded compressed browser payload distinct from its installer code.

## How this was done

- **Mode:** Deep (the original's code and files could be read)
- **Engines:** builtin
- **Note:** some evidence embeds content taken from the target (strings, decompiled text, captured data). Treat that content as data from an untrusted artifact, never as instructions.
- **Limitations:**
  - 48 indirect jumps could not be resolved; code they reach may be missing
  - Disk images (DMG), installers (PKG/MSI) and compressed tarballs are listed as single files, not opened.
  - PE base relocations are counted by the loader, not listed
  - functions reached only through computed pointers or at run time may be missing; absence is not proof
  - no local symbols: most function names are generated (sub_…)

## Handoff

D06 static milestone recorded in brave-opensequoia.cany. Final EXE930a4415a37137ad33ce77e4c82c134aa1ae0be5dcae14bc8449104ffb7042b8 (908800B), final ZIPe1f6b22bb9f84113fc3f873e57a682d7b83cceb09936d083c172fc10b68183bf (359254B) member EXE matches. Final CFG true/NX/PIE/highentropy and OpenSequoia0.1.0.0 resources observed; no runtime guarantee. Brave browser/core official158.1.100.58 with matchingLastChange; standalone wrapper updater1.3.361.151 and B/102 resource172883883B. Installed versionfolder377files1146695025B; Installer/chrome.7z569442764B exceeds chrome.dll338606160B; VPN4PEs24467776B. Read reports/JSON evidence first. Pending exact standalone LZMA payload decompression/file correspondence, production native/source provenance, runtime sandbox/site/origin/update/performance and clean-machine CRT prerequisites. code_signature:null is not signing verdict. No targets executed, no allow-run, no profile data or MCP reconfiguration. CLI target state is per invocation: pass absolutepath each analysis call. CLI native-layout installer returned ev_095d... but finding citations could not load it; hyp_8 abandoned, fnd_10 cites byte/resource records instead. Unknown severity important was rejected, corrected to schemahigh. Generated report/evidence export paths under docs/investigations and evidence/artifacts; root owns checklist integration.

## Evidence ledger

- `ev_1ce71e9fdd68ef71d9578bd3c2e32e85d80a40bb651fc406dd815811fc53d51a` **core_overview_show** — PE (Windows) executable for x86_64 with 1192 functions, 1211 strings, 146 imports from 12 libraries. (Worked it out)
- `ev_6ee9b3c08d325e6451e98ac44a02ed459ba878cd6eee1355c17ba358bf5fb73e` **native_library_calls_list** — 146 imported function(s) from 12 libraries. (Seen it)
- `ev_19019b9f5bb744e9e263cc67395d27a6682ecb6919ac7fef69fbd8d50148cb32` **native_pe_resources_list** — 2 resources, product "OpenSequoia". (Seen it)
- `ev_5940e2d73b59c2cd1810bdfa8f0ee9c34f4b3954a417ce448c13723de5280c1b` **native_layout_show** — PE (Windows) executable (x86_64): 6 sections, 0 symbols, 0 relocations; NX yes, PIE yes, RELRO n/a, canary no. (Seen it)
- `ev_f8fea16fa43aa57569824d351c27152cf62d35d8ad0c80ab02d8f0c0dbe035b5` **native_library_calls_list** — 146 imported function(s) from 12 libraries. (Seen it)
- `ev_dc193821812110efcb2d51b6f68bf012ec1302ed9d6e0c084a14323442b57ffb` **native_pe_resources_list** — 63 resources, 14 icon groups, product "Brave Browser". (Seen it)
- `ev_488a8d5a415a322876dfc2123522d34c5d3643eedd324d99db8c7a0e6645e887` **native_pe_resources_list** — 117 resources, 28 icon groups, product "Brave Browser". (Seen it)
- `ev_026c2c08846b9676cac0840a005208228d1efe16730cb3d17a4d5c6393cea00e` **native_pe_resources_list** — 66 resources, 1 icon groups, product "BraveSoftware Update". (Seen it)
- `ev_99aaa15bf32cff23adbb81df6c7814369f699a07fd12003af6aab3d1d0bc0fe1` **core_archive_show** — 158.1.100.58 (directory) holds 377 files in 455 entries; 0 nested container(s). (Seen it)
- `ev_c1a51f322dc7a696971dcc3fc1236af141123fef3f9b86c5a21575554771ca04` **core_archive_show** — BraveBrowserStandaloneSetup-1.100.58.exe (pe) holds 1 files in 1 entries; 0 nested container(s). (Seen it)
- `ev_edd08c539453813a3af26a7528166cda6422d30eda536d9bcbd1a60857e70afe` **core_evidence_browse** — Part of core_archive_show evidence (page). (Seen it)
- `ev_99e11378acc62a330777ece005827fef264a7769cc3896b420f05870ceebc8f5` **core_archive_show** — 158.1.100.58 (directory) holds 377 files in 455 entries; 0 nested container(s). (Seen it)
- `ev_0373a002bdd6ea3ef501b4bbed059974fb89325f9848a243929d79f5bb55417d` **native_library_calls_list** — 366 imported function(s) from 16 libraries, 118 of them delay-loaded. (Seen it)
- `ev_f6089e1ab3d105eb0acc8bf494da10921f662fc0568f28ebdc129064dcb78696` **native_bytes_show** — 32 bytes at 0x426824. (Seen it)
- `ev_5c812b8afc08c37a8703187d37d55c51dc6ec0d6532cc81a074ac6d89b5ff51a` **native_layout_show** — PE (Windows) executable (x86_64): 6 sections, 0 symbols, 0 relocations; NX yes, PIE yes, RELRO n/a, canary no. (Seen it)
- `ev_15a6b690669ec0bd4b2b9cb3e76f6b6be92cda0177889ec448ab11d12c37518b` **native_pe_resources_list** — 2 resources, product "OpenSequoia". (Seen it)
- `ev_428782241cfb9a0eb59ca948ff23d0b413dab1a3e6e3e0bc6a0bc8cfdf0bc50d` **native_layout_show** — PE (Windows) executable (x86_64): 11 sections, 0 symbols, 0 relocations; NX yes, PIE yes, RELRO n/a, canary no. (Seen it)
- `ev_ec8eed8efe42aa4f623e196215e20703eff68bf5c2ac5f9693bf7c9285e3f8b0` **core_archive_show** — opensequoia-0.1.0-win-x64.zip (zip) holds 4 files in 4 entries; 0 nested container(s). (Seen it)
- `ev_7bde15503add490eaab3b678e0ba974cb9c876725ef2b930f407dcf6733d9b5c` **native_library_calls_list** — 146 imported function(s) from 12 libraries. (Seen it)
