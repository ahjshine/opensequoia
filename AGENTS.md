# OpenSequoia working rules

Read SPEC.md, CONSTRAINTS.md, ARCHITECTURE.md and TASKS.md before writing code. Do not weaken constraints to make a change pass. User mission is authoritative.

Rust application and engine only; no Chromium/Blink/V8/CEF/Electron/WebView2 fallback. Document all bundled native implementation; documented Windows OS services are allowed. Safe Rust by default; unsafe requires invariants and independent review.

Root owns Cargo manifests/lockfile, dependency decisions, root records, acceptance and commits. Shared checkout uses explicit ownership in TASKS.md and delegated briefs. Do not change files outside ownership without coordination. No agent may merge or claim parity from a shell or subset demo.

Every completed item needs reproducible evidence, limits and a handoff. Keep required tests and gaps visible. Never include credentials or personal browsing data in logs. Artifact questions use the installed Craft Anything workflow; source questions use pinned direct source inspection.
