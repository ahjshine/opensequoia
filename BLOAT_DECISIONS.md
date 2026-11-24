# Optional functionality decisions

Source pin and classification evidence: FEATURE_PARITY.md and docs/discovery/brave.md. This is a fresh Rust implementation, so exclusions are **not imported**, rather than claiming deletions from a Brave binary. Shipped absence requires later artifact/dependency/runtime verification. Missing strings cannot establish absence.

| Decision | Verified feature | Policy rationale | Dependency consequence / verification |
|---|---|---|---|
| B01 exclude | Wallet/Web3 at components/brave_wallet | unrelated crypto/service functionality | Do not remove retained sync crypto merely for BAT/bip39 names; replace its required native crypto with compliant Rust |
| B02 exclude | Rewards, Brave Ads, sponsored new tab images | advertising monetization/sponsored placements | preserve networking/settings/generic new tab infrastructure; no ad service keys/resources |
| B03 exclude | Leo/ai_chat | optional bundled assistant | preserve general browser UI, networking and developer functionality |
| B04 exclude | Brave VPN subscription | optional subscription integration | retain secure network/proxy/permission capabilities needed by browser |
| B05 exclude | Brave News/Talk/SKU/referral service integrations | optional news/conferencing commerce/promotion | retain normal navigation/media/standards payments functionality; no inference all SKU-linked common code optional |
| B06 exclude | P3A and unnecessary marketing/usage telemetry | optional telemetry, privacy/footprint policy | retain privacy-preserving local diagnostics and vulnerability/crash handling required by reliability |
| B07 investigate | Shopping/referral candidates beyond inspected paths | exclusion authorized only where verified | do not invent presence; finish exact source/service inventory before new exclusion |
| B08 retain pending feasibility | Tor private windows | security/privacy, not optional monetization by default | determine Rust/protocol/service/license approach; native Tor bundle not silently adopted; unavailable stays blocker |
| B09 retain | Sync, extensions, password/autofill, accessibility, developer tools | expressly essential scope | substantial implementation/backend/licensing work remains; never remove for size or RAM |
| B10 supporting infrastructure pending usage review | Installed Installer/chrome.7z cache, observed 569442764 bytes | Largest footprint contributor, but not optional merely because it is large | Determine install/update/repair/rollback dependencies before changing retention policy; keep recovery essential |

Craft's installed Brave inventory confirms four VPN-related PE files totalling 24467776 bytes. This establishes packaged presence at the selected installation snapshot, not activation, subscription state or a measured OpenSequoia savings claim. Baseline wrapper executable is version 1.3.361.151, with a large B/102 resource; do not mistake updater-wrapper version for browser payload version. Detailed findings/evidence/unknowns are in docs/investigations.

No quantified savings are claimed before equivalent measurements. Shared dependencies are mapped before exclusion. Final release requires Craft packaged resource/import/dependency analysis and runtime behavior checks; source intent alone cannot prove package policy.

Craft's 0.2 GUI package finding fnd_21 observes exactly four ZIP members (EXE, LICENSE, README, notices), bound to source checkpoint83451aa and the GUI hash. Resource finding fnd_19 covers the complete two-resource PE tree (version and manifest). No separate optional-service payload is present in those inventories; this does not establish absence of linked or dynamically loaded code, service calls, or native implementation. Those boundaries and unknowns remain in gui2-handoff.md and the dependency/parity records.
