# S03c/R04 root resume and gate state

Date: 2026-11-24 (Asia/Calcutta). Integrated branch: `crew/controller`.

## Changes applied

- `98d760c` integrates the reviewed gate fix: the sandbox-smoke arm is cfg-gated for non-Windows builds, and the acceptance oracle requires parent-owned markers printed only after the child token/file/network evidence validates.
- `54289fd` splits the parent observations so earned file-denial and unchanged-fixture evidence is still reported when the later loopback probe fails. It does not relax the network or final-smoke assertions.
- R04's review is integrated at `docs/reviews/appcontainer-probe.md` with a post-review addendum; its original verdict and `b3e3ccc` binding are preserved.

## Actual acceptance result

Command: `python -B evidence/verification/sandbox4-acceptance.py`.

Result: **24 assertions passed, 4 unmet**; no `S03C_PROBE_OK` and no success claim.

Unmet:

- `sandbox smoke exit`
- `actual helper loopback WSAEACCES 10013`
- `broker exact denial/token observation`
- `complete sandbox smoke`

The child reported `SANDBOX_PROBE_ERROR operation=Winsock WSAPoll loopback connect timed out with SO_ERROR=0`. File denials (`read=5 write=5`), unchanged fixture, zero accepted sandbox connections, token/SID/Job/handle checks, lifetime, parent-death recovery and cleanup all passed. Log: `evidence/verification/sandbox4-acceptance.log`.

## Diagnosis and decision boundary

R04 and the independent host-behaviour note found no source defect explaining the missing `10013`: the ordinary control connects, the AppContainer connect is never accepted, and the host silently drops the attempt. The current Windows build's AppContainer/WFP policy therefore does not supply the specific Winsock error the owner's gate requires.

No criterion, capability, firewall rule or loopback exemption was changed. The owner must choose one of:

1. Keep the red gate and record `10013` as host-version-qualified; S03c/R04 stay blocked where the host drops silently.
2. Re-scope the assertion to the security invariant (ordinary control accepted; sandbox connect not accepted; zero accepts; fixture unchanged) while logging the exact timeout/error separately. This is an owner decision, not a reviewer or worker change.
3. First obtain an elevated WFP filter enumeration showing the non-terminating block, then re-scope on that evidence.

Until that decision, then a fresh independent review of the fixed tree, S03c and R04 remain **BLOCKED** and remote input stays disabled.