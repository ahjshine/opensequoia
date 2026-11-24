#!/usr/bin/env python3
"""R04 read-only S03c source/evidence/report consistency checker.

This does not execute product probes or manufacture a gstack receipt. It binds the
review to exact source and evidence hashes. SANDBOX_REVIEW_OK is emitted only for
consistent evidence with zero unresolved CRITICAL findings, zero unmet required
assertions, and CLEAN verdicts in both human reports.
"""
from __future__ import annotations

import hashlib
import json
from pathlib import Path
import re
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[2]
CREW = ROOT.parent
OUT_JSON = ROOT / "evidence" / "verification" / "sandbox-review.json"
OUT_LOG = ROOT / "evidence" / "verification" / "sandbox-review.log"
REPORT = ROOT / "docs" / "reviews" / "appcontainer-probe.md"
SUMMARY = CREW / "review" / "r04-summary.md"
FINDINGS = CREW / "review" / "r04-findings.jsonl"
ACCEPTANCE_JSON = ROOT / "evidence" / "verification" / "sandbox4-acceptance.json"
ACCEPTANCE_LOG = ROOT / "evidence" / "verification" / "sandbox4-acceptance.log"
PROBE_LOG = ROOT / "target" / "r04" / "direct-drop.log"

SNAPSHOT = "b3e3ccc0d3f413747ca7c3be76f34761895c8094"
EVIDENCE_REVISION = "22abfd5604c13e97a89ea416efd2154e75c50257"
SOURCE_HASHES = {
    "crates/app/src/windows_sandbox.rs": "1ae44f9d316e4796cd88faceba48b412fb86e9d45f86164c7b6955f0195efc97",
    "crates/app/src/windows_process.rs": "65f42fea9b302858076d5046767df11b708b0a9c97e6689fd5ac05bac5cd246d",
    "crates/app/src/worker.rs": "802f6c5274cff59711da028d0e2ea4e38306638b6a823791fa1431d88fb09b1d",
    "crates/app/src/main.rs": "2c06b3551678656d80e4fac74989cb003c33a73686c91babaf3569d2cf700d52",
    "crates/app/src/input.rs": "25106af641986a62785d3b7f6bc2ce540e2031f6a921cf7dacadb3bb2be15a60",
    "crates/app/Cargo.toml": "e8d6ef76b14e277613b4acc1d02bd3437b7b1046da0f757b947b596c338c49cf",
    "Cargo.toml": "1caebaa3f3b1ce49737f390baad18c1daa33a2716cbdf1c0963d357ad12548dd",
    "Cargo.lock": "e801e3ec0b12e1d134834a119102c4ac270d328818683c5c7cdb4d11b5852707",
    "evidence/verification/sandbox4-acceptance.py": "f71a782718153f585aef611a3fb91fbf79bb7f32149537b9a112b40f9b1c82b3",
    "evidence/verification/sandbox4-process-helper.rs": "6f972a377381fd158a2004dccd67f240bb425e61bf19fb32bb2e474690ea7c72",
}
EVIDENCE_HASHES = {
    "evidence/verification/sandbox4-acceptance.json": "dd9ee9c9415ab8bd1bfc19369a12f9a85fdba6483279785209d24dcfdee9754c",
    "evidence/verification/sandbox4-acceptance.log": "15aa1b4ecf0d3233ae0f80ca731dfe369617c8cebe208601a8526b34c169be92",
    "target/r04/direct-drop-probe.rs": "907e37a6b368c5b0350b45895b86bc6caefb11da70d849b6ae02cef785b84891",
    "target/r04/direct-drop.log": "2f2f4d71287fbaae4c69c54af80496ef66a981f26c992dff6031d4a62a12bf62",
    "target/r04/scoped.diff": "3a51acec85a882d9431d6657822458442d55c5358275636c05cef0b9229be017",
}
EXPECTED_UNMET = [
    "sandbox smoke exit",
    "actual helper loopback WSAEACCES 10013",
    "broker exact denial/token observation",
    "complete sandbox smoke",
]
EXPECTED_ASSERTIONS = [
    ("no preexisting generated staging", True),
    ("application build", True),
    ("one compatible windows-sys rlib", True),
    ("process helper build", True),
    ("ordinary process smoke exit", True),
    ("ordinary healthy/malformed/restart/reap controls", True),
    ("sandbox smoke exit", False),
    ("real renderer equality and suspended token policy", True),
    ("ordinary file/network positive control", True),
    ("AppContainer exact file denials", True),
    ("unchanged fixture and no accepted sandbox connection", True),
    ("actual helper loopback WSAEACCES 10013", False),
    ("broker exact denial/token observation", False),
    ("observable sandbox profile/staging cleanup", True),
    ("complete sandbox smoke", False),
    ("lifetime exit", True),
    ("wrong-path fail closed", True),
    ("Job close kills real sandbox child", True),
    ("two-process descendant positive control", True),
    ("one-process quota 1816", True),
    ("included handle marker positive control", True),
    ("excluded strict-handle observation", True),
    ("real pipe cancellation 995", True),
    ("lifetime summary", True),
    ("lifetime reaping and cleanup", True),
    ("parent-death exit", True),
    ("parent death kills child and recovers owned profile/staging", True),
    ("all generated staging removed", True),
]
EXPECTED_COMMANDS = [
    ("build application", 0, "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855", "56506dbfdd30899fe8a159d892b69276dbcf98ab0ac52d699cd0dffea2bb9c95"),
    ("build real process helper", 0, "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855", "0140ef8ac7d413709ec963efa01281f9fe9fc3498aabadb3bc068be7f0666e5a"),
    ("ordinary process smoke", 0, "afd7aa97f57552b59f42b0ea2caaaa1eb3c16373a1f1f373c85c0fcd51d10b0f", "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"),
    ("AppContainer renderer and denial smoke", 1, "c3befc26d26a7fbc9e25ed4cf0fb9c94ac263968cadcda8ad631f0c374b267ad", "336d603e72d580dcdbc80b5d2ec7f796eeb09abc4c3b41307f9c7ed6b052c314"),
    ("AppContainer Job/handle/cancellation lifetime", 0, "87f899b08328717b24c6d263a558279786f49176ece039c149e9bcf8fdbd0677", "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"),
    ("AppContainer parent death and owned recovery", 0, "4e5188fed36c54edddb1dee24131c0c054ea536b5ac29728185dbde1b25f268a", "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"),
]


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def report_fields(text: str) -> dict[str, str]:
    fields: dict[str, str] = {}
    for line in text.splitlines():
        match = re.fullmatch(r"(R04_[A-Z_]+)=(.*)", line)
        if match:
            key, value = match.groups()
            if key in fields:
                raise ValueError(f"duplicate report field {key}")
            fields[key] = value
    return fields


def main() -> tuple[dict, list[str], bool]:
    checks: list[dict] = []
    actual_sources: dict[str, str] = {}
    actual_evidence: dict[str, str] = {}

    def check(name: str, passed: bool, detail: str) -> None:
        checks.append({"name": name, "passed": bool(passed), "detail": detail})

    for relative, expected in SOURCE_HASHES.items():
        actual = sha256(ROOT / relative)
        actual_sources[relative] = actual
        check(f"source hash {relative}", actual == expected, actual)
    for relative, expected in EVIDENCE_HASHES.items():
        actual = sha256(ROOT / relative)
        actual_evidence[relative] = actual
        check(f"evidence hash {relative}", actual == expected, actual)

    head = subprocess.run(
        ["git", "rev-parse", "HEAD"], cwd=ROOT, capture_output=True,
        text=True, encoding="utf-8", timeout=15, check=False,
    )
    actual_head = head.stdout.strip()
    check("snapshot HEAD", head.returncode == 0 and actual_head == SNAPSHOT, actual_head)
    unchanged = subprocess.run(
        ["git", "diff", "--quiet", f"{EVIDENCE_REVISION}..{SNAPSHOT}", "--",
         "crates/engine/src", "crates/ipc/src", "fixtures", ".cargo"],
        cwd=ROOT, capture_output=True, text=True, encoding="utf-8", timeout=15, check=False,
    )
    check(
        "post-evidence production engine/IPC/fixture inputs unchanged",
        unchanged.returncode == 0,
        f"git diff --quiet returncode={unchanged.returncode}",
    )

    acceptance = json.loads(ACCEPTANCE_JSON.read_text(encoding="utf-8"))
    acceptance_log = ACCEPTANCE_LOG.read_text(encoding="utf-8")
    check("acceptance schema", acceptance.get("schema") == "opensequoia-s03c-acceptance-v1", str(acceptance.get("schema")))
    check("acceptance execution revision recorded", acceptance.get("revision") == EVIDENCE_REVISION, str(acceptance.get("revision")))
    integration = acceptance.get("integration", {})
    check("integrated evidence kind", integration.get("kind") == "integrated_checkout", str(integration.get("kind")))
    for relative in ("crates/app/src/windows_sandbox.rs", "crates/app/src/windows_process.rs", "crates/app/src/worker.rs", "evidence/verification/sandbox4-acceptance.py", "evidence/verification/sandbox4-process-helper.rs"):
        check(f"acceptance lane hash {relative}", integration.get("lane_sources", {}).get(relative) == SOURCE_HASHES[relative], str(integration.get("lane_sources", {}).get(relative)))
    for relative in ("Cargo.toml", "Cargo.lock", "crates/app/Cargo.toml", "crates/app/src/main.rs", "crates/app/src/input.rs"):
        check(f"acceptance root hash {relative}", integration.get("root_files", {}).get(relative) == SOURCE_HASHES[relative], str(integration.get("root_files", {}).get(relative)))

    actual_assertions = [(item.get("name"), item.get("passed")) for item in acceptance.get("assertions", [])]
    check("exact 28 assertion identities/states", actual_assertions == EXPECTED_ASSERTIONS, f"count={len(actual_assertions)}")
    passed_count = sum(1 for _, passed in EXPECTED_ASSERTIONS if passed)
    check("24 passing and four unmet", passed_count == 24 and len(EXPECTED_UNMET) == 4, f"passed={passed_count} unmet={len(EXPECTED_UNMET)}")
    check("acceptance overall failed", acceptance.get("passed") is False, str(acceptance.get("passed")))
    check("exact four unmet names", acceptance.get("unmet") == EXPECTED_UNMET, json.dumps(acceptance.get("unmet")))

    actual_commands = [
        (item.get("label"), item.get("returncode"), item.get("stdout_sha256"), item.get("stderr_sha256"))
        for item in acceptance.get("commands", [])
    ]
    check("exact command results and stream hashes", actual_commands == EXPECTED_COMMANDS, f"commands={len(actual_commands)}")

    runtime = acceptance_log.partition("PASS no preexisting generated staging:")[0]
    required_runtime = [
        "SANDBOX_RENDER app=1 sid_equal=1 integrity_rid=4096 capabilities=0 equal=1 spawned=2 reaped=2",
        "SANDBOX_DENIAL_CONTROL app=0 read=0 write=0 network=0 accepted=1 fixture_unchanged=1",
        "SANDBOX_PROBE_FILES app=1 read=5 write=5",
        "SANDBOX_DENIAL_OBSERVATION accepted=0 fixture_unchanged=1",
        "SANDBOX_PROBE_ERROR operation=Winsock WSAPoll loopback connect timed out with SO_ERROR=0",
        "SANDBOX_LIFETIME_STAGE handle_include=true result=INHERITED",
        "SANDBOX_LIFETIME_STAGE handle_include=false result=0xc0000008",
        "SANDBOX_PARENT_DEATH killed=1 child_reaped=1 profile_deleted=1 staging_removed=1",
    ]
    for marker in required_runtime:
        check(f"runtime marker {marker.split()[0]}", marker in runtime, marker)
    runtime_lines = {line.strip() for line in runtime.splitlines()}
    check("no actual exact 10013 helper marker", "SANDBOX_PROBE app=1 read=5 write=5 network=10013" not in runtime_lines, "timeout retained")
    check("no actual sandbox smoke success", not any(line.startswith("SANDBOX_SMOKE_OK ") for line in runtime_lines), "SANDBOX_SMOKE_OK absent")
    check("no acceptance success token", "S03C_PROBE_OK" not in acceptance_log, "S03C_PROBE_OK absent")

    probe = PROBE_LOG.read_text(encoding="utf-8")
    check(
        "focused final-owner probe result",
        "DIRECT_DROP_OBSERVATION immediate_root=0 immediate_binary=0 child_reaped=1 recovery=not_needed final_root=0" in probe
        and "remaining_run_entries=0" in probe,
        "direct drop cleaned; one host only",
    )

    findings = []
    for raw in FINDINGS.read_text(encoding="utf-8").splitlines():
        if raw.strip():
            findings.append(json.loads(raw))
    for index, finding in enumerate(findings):
        required = {"severity", "confidence", "path", "line", "category", "summary", "fix", "fingerprint", "specialist"}
        valid = required <= finding.keys()
        valid = valid and finding.get("severity") in {"CRITICAL", "INFORMATIONAL"}
        valid = valid and isinstance(finding.get("confidence"), int) and 0 <= finding["confidence"] <= 10
        valid = valid and isinstance(finding.get("line"), int) and finding["line"] > 0
        valid = valid and finding.get("category") == "security" and finding.get("specialist") == "security"
        valid = valid and finding.get("fingerprint") == f"{finding.get('path')}:{finding.get('line')}:security"
        valid = valid and finding.get("path") in SOURCE_HASHES
        check(f"finding {index} schema/fingerprint", valid, str(finding.get("fingerprint")))
    critical = sum(item["severity"] == "CRITICAL" for item in findings)
    informational = sum(item["severity"] == "INFORMATIONAL" for item in findings)
    check("finding inventory", critical == 1 and informational == 1 and len(findings) == 2, f"critical={critical} informational={informational}")

    report_text = REPORT.read_text(encoding="utf-8")
    summary_text = SUMMARY.read_text(encoding="utf-8")
    report = report_fields(report_text)
    summary = report_fields(summary_text)
    expected_verdict = "BLOCKED" if critical or len(EXPECTED_UNMET) else "CLEAN"
    expected_report = {
        "R04_VERDICT": expected_verdict,
        "R04_CRITICAL": str(critical),
        "R04_INFORMATIONAL": str(informational),
        "R04_REQUIRED_UNMET": str(len(EXPECTED_UNMET)),
        "R04_ACCEPTANCE_PASSED": "24",
        "R04_NEW_IMPLEMENTATION_DEFECTS": "0",
        "R04_SNAPSHOT": SNAPSHOT,
    }
    for key, expected in expected_report.items():
        check(f"main report {key}", report.get(key) == expected, str(report.get(key)))
    for key in ("R04_VERDICT", "R04_CRITICAL", "R04_INFORMATIONAL", "R04_REQUIRED_UNMET", "R04_SNAPSHOT"):
        check(f"summary agrees {key}", summary.get(key) == expected_report[key], str(summary.get(key)))

    consistent = all(item["passed"] for item in checks)
    clean = consistent and critical == 0 and not EXPECTED_UNMET and expected_verdict == "CLEAN"
    result = {
        "schema": "opensequoia-r04-sandbox-review-v1",
        "snapshot": actual_head,
        "evidence_execution_revision": EVIDENCE_REVISION,
        "source_hashes": {path: {"expected": SOURCE_HASHES[path], "actual": actual_sources[path]} for path in SOURCE_HASHES},
        "evidence_hashes": {path: {"expected": EVIDENCE_HASHES[path], "actual": actual_evidence[path]} for path in EVIDENCE_HASHES},
        "report_sha256": sha256(REPORT),
        "summary_sha256": sha256(SUMMARY),
        "findings_sha256": sha256(FINDINGS),
        "acceptance": {"passed_assertions": 24, "unmet": EXPECTED_UNMET, "passed": acceptance.get("passed")},
        "findings": {"critical": critical, "informational": informational},
        "checks": checks,
        "evidence_consistent": consistent,
        "clean": clean,
        "passed": clean,
        "verdict": "CLEAN" if clean else "BLOCKED",
        "limits": [
            "No product probe or broad suite was rerun by this checker.",
            "The known loopback 10013 gate remains unmet; timeout/SO_ERROR=0 is not denial.",
            "No formal gstack receipt, READY, production sandbox, or remote enablement is established.",
        ],
    }
    lines = [f"{'PASS' if item['passed'] else 'FAIL'} {item['name']}: {item['detail']}" for item in checks]
    lines.append(f"evidence_consistent={consistent} critical={critical} required_unmet={len(EXPECTED_UNMET)}")
    return result, lines, clean


def write_and_print(result: dict, lines: list[str], clean: bool) -> int:
    OUT_JSON.write_text(json.dumps(result, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    output = ["checker=python evidence/verification/sandbox-review-check.py", *lines]
    if clean:
        output.append("SANDBOX_REVIEW_OK")
    else:
        output.append(
            f"SANDBOX_REVIEW_BLOCKED unresolved_critical={result.get('findings', {}).get('critical', 'unknown')} "
            f"required_unmet={len(result.get('acceptance', {}).get('unmet', []))}"
        )
    OUT_LOG.write_text("\n".join(output) + "\n", encoding="utf-8")
    print("\n".join(output))
    return 0 if clean else 1


if __name__ == "__main__":
    try:
        review_result, review_lines, review_clean = main()
    except Exception as error:
        failure = {
            "schema": "opensequoia-r04-sandbox-review-v1",
            "passed": False,
            "clean": False,
            "verdict": "BLOCKED",
            "harness_error": f"{type(error).__name__}: {error}",
        }
        code = write_and_print(failure, [f"HARNESS_ERROR {type(error).__name__}: {error}"], False)
        sys.exit(2 if code else 2)
    sys.exit(write_and_print(review_result, review_lines, review_clean))
