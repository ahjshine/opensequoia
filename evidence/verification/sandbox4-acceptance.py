#!/usr/bin/env python3
"""S03c real AppContainer acceptance oracle.

Stdlib-only, synthetic local data and loopback only. Builds in target/sandbox4,
records every required assertion, and never converts launch/path/timeout errors
into denial success. An integrated checkout is used directly. A lane checkout
without root integration uses OPENSEQUOIA_S03C_INTEGRATION_ROOT read-only and
creates a test-only integration copy under target/sandbox4.

Value: protects=real token/file/network/lifecycle gates; fails_when=a positive
control or exact denial is absent; why_new=mocks cannot observe Windows security
boundaries; seam=none; no_claim=production sandbox or other hosts.
"""

from __future__ import annotations

import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import time
from datetime import datetime, timezone

REPO = Path(__file__).resolve().parents[2]
TARGET = REPO / "target" / "sandbox4"
JSON_PATH = REPO / "evidence" / "verification" / "sandbox4-acceptance.json"
LOG_PATH = REPO / "evidence" / "verification" / "sandbox4-acceptance.log"
OWNED_SOURCES = [
    "crates/app/src/windows_sandbox.rs",
    "crates/app/src/windows_process.rs",
    "crates/app/src/worker.rs",
    "evidence/verification/sandbox4-process-helper.rs",
    "evidence/verification/sandbox4-acceptance.py",
]
ROOT_FILES = [
    "Cargo.toml",
    "Cargo.lock",
    "crates/app/Cargo.toml",
    "crates/app/src/main.rs",
    "crates/app/src/input.rs",
]
MAX_OUTPUT = 65_536
COMMANDS: list[dict] = []
ASSERTIONS: list[dict] = []
LOG_LINES: list[str] = []


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def bounded(text: str) -> tuple[str, bool]:
    encoded = text.encode("utf-8", errors="replace")
    if len(encoded) <= MAX_OUTPUT:
        return text, False
    return encoded[:MAX_OUTPUT].decode("utf-8", errors="replace"), True


def run_command(
    label: str,
    arguments: list[str],
    cwd: Path,
    timeout: int = 180,
    minimal_environment: bool = False,
) -> dict:
    environment = None
    if minimal_environment:
        environment = {
            name: os.environ[name]
            for name in ("SystemRoot", "WINDIR", "SystemDrive")
            if name in os.environ
        }
    started = time.monotonic()
    record = {
        "label": label,
        "arguments": arguments,
        "cwd": str(cwd),
        "timeout_seconds": timeout,
    }
    try:
        completed = subprocess.run(
            arguments,
            cwd=cwd,
            env=environment,
            stdin=subprocess.DEVNULL,
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace",
            timeout=timeout,
            check=False,
        )
        record.update(
            returncode=completed.returncode,
            timed_out=False,
            stdout=completed.stdout,
            stderr=completed.stderr,
        )
    except subprocess.TimeoutExpired as error:
        record.update(
            returncode=None,
            timed_out=True,
            stdout=error.stdout or "",
            stderr=error.stderr or "",
        )
    record["duration_seconds"] = round(time.monotonic() - started, 3)
    record["combined"] = record["stdout"] + record["stderr"]
    COMMANDS.append(record)
    return record


def assertion(name: str, passed: bool, evidence: str) -> None:
    ASSERTIONS.append({"name": name, "passed": bool(passed), "evidence": evidence})


def require_marker(label: str, record: dict, marker: str) -> None:
    assertion(label, marker in record.get("combined", ""), marker)


def integrated_root() -> tuple[Path, dict]:
    main = REPO / "crates" / "app" / "src" / "main.rs"
    options = REPO / "crates" / "app" / "src" / "input.rs"
    manifest = REPO / "crates" / "app" / "Cargo.toml"
    if (
        "mod windows_sandbox;" in main.read_text(encoding="utf-8")
        and "SandboxSmoke" in options.read_text(encoding="utf-8")
        and "Win32_Security_Isolation" in manifest.read_text(encoding="utf-8")
    ):
        return REPO, {
            "kind": "integrated_checkout",
            "root": str(REPO),
            "root_files": {path: sha256(REPO / path) for path in ROOT_FILES},
        }

    controller_value = os.environ.get("OPENSEQUOIA_S03C_INTEGRATION_ROOT")
    if not controller_value:
        raise RuntimeError(
            "lane lacks root integration; set OPENSEQUOIA_S03C_INTEGRATION_ROOT "
            "to the controller-verified read-only checkout"
        )
    controller = Path(controller_value).resolve()
    overlay = TARGET / "oracle-integration"
    overlay.mkdir(parents=True, exist_ok=True)
    for name in ("Cargo.toml", "Cargo.lock"):
        shutil.copy2(REPO / name, overlay / name)
    for name in (".cargo", "crates", "fixtures"):
        shutil.copytree(REPO / name, overlay / name, dirs_exist_ok=True)
    helper_destination = overlay / "evidence" / "verification" / "sandbox4-process-helper.rs"
    helper_destination.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(REPO / "evidence" / "verification" / "sandbox4-process-helper.rs", helper_destination)
    hashes = {}
    for relative in ROOT_FILES:
        source = controller / relative
        destination = overlay / relative
        destination.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(source, destination)
        source_hash = sha256(source)
        if sha256(destination) != source_hash:
            raise RuntimeError(f"integration copy hash mismatch: {relative}")
        hashes[relative] = source_hash
    return overlay, {
        "kind": "test_only_controller_overlay_not_root_gate_evidence",
        "root": str(controller),
        "overlay": str(overlay),
        "root_files": hashes,
    }


def main() -> int:
    TARGET.mkdir(parents=True, exist_ok=True)
    revision = subprocess.run(
        ["git", "rev-parse", "HEAD"],
        cwd=REPO,
        capture_output=True,
        text=True,
        encoding="utf-8",
        check=True,
    ).stdout.strip()
    build_root, provenance = integrated_root()
    provenance["lane_sources"] = {
        relative: sha256(REPO / relative) for relative in OWNED_SOURCES
    }
    runs = TARGET / "runs"
    preexisting = list(runs.iterdir()) if runs.exists() else []
    assertion(
        "no preexisting generated staging",
        not preexisting,
        f"entries={len(preexisting)}; oracle never deletes abandoned input",
    )
    if preexisting:
        raise RuntimeError("generated staging already exists; inspect it before rerunning")

    build = run_command(
        "build application",
        [
            "cargo",
            "build",
            "--manifest-path",
            str(build_root / "Cargo.toml"),
            "--target-dir",
            str(TARGET / "oracle-build"),
            "-p",
            "opensequoia",
            "--locked",
            "--offline",
        ],
        build_root,
    )
    assertion("application build", build["returncode"] == 0, f"returncode={build['returncode']}")

    rlibs = []
    if build["returncode"] == 0:
        rlibs = list((TARGET / "oracle-build" / "debug" / "deps").glob("libwindows_sys-*.rlib"))
    assertion("one compatible windows-sys rlib", len(rlibs) == 1, f"count={len(rlibs)}")
    helper_build = None
    if len(rlibs) == 1:
        helper_build = run_command(
            "build real process helper",
            [
                "rustc",
                "--edition=2024",
                str(REPO / "evidence" / "verification" / "sandbox4-process-helper.rs"),
                "--extern",
                f"windows_sys={rlibs[0]}",
                "-L",
                f"dependency={TARGET / 'oracle-build' / 'debug' / 'deps'}",
                "-o",
                str(TARGET / "oracle-process-helper.exe"),
            ],
            REPO,
        )
        assertion(
            "process helper build",
            helper_build["returncode"] == 0,
            f"returncode={helper_build['returncode']}",
        )
    else:
        assertion("process helper build", False, "not run without one compatible rlib")

    executable = TARGET / "oracle-build" / "debug" / "opensequoia.exe"
    helper = TARGET / "oracle-process-helper.exe"
    if build["returncode"] == 0:
        ordinary = run_command(
            "ordinary process smoke",
            [str(executable), "--process-smoke"],
            build_root,
            timeout=60,
            minimal_environment=True,
        )
        assertion("ordinary process smoke exit", ordinary["returncode"] == 0, f"returncode={ordinary['returncode']}")
        require_marker(
            "ordinary healthy/malformed/restart/reap controls",
            ordinary,
            "PROCESS_SMOKE_OK healthy=1 malformed_rejected=1 restarted=1 spawned=3 reaped=3",
        )
        sandbox = run_command(
            "AppContainer renderer and denial smoke",
            [str(executable), "--sandbox-smoke"],
            build_root,
            timeout=90,
            minimal_environment=True,
        )
        assertion("sandbox smoke exit", sandbox["returncode"] == 0, f"returncode={sandbox['returncode']}")
        required_sandbox_markers = {
            "real renderer equality and suspended token policy": "SANDBOX_RENDER app=1 sid_equal=1 integrity_rid=4096 capabilities=0 equal=1 spawned=2 reaped=2",
            "ordinary file/network positive control": "SANDBOX_DENIAL_CONTROL app=0 read=0 write=0 network=0 accepted=1 fixture_unchanged=1",
            "AppContainer exact file denials": "SANDBOX_PROBE_VERIFIED_FILES app=1 read=5 write=5",
            "unchanged fixture and no accepted sandbox connection": "SANDBOX_DENIAL_OBSERVATION accepted=0 fixture_unchanged=1",
            "actual helper loopback WSAEACCES 10013": "SANDBOX_PROBE_VERIFIED app=1 read=5 write=5 network=10013",
            "broker exact denial/token observation": "SANDBOX_DENIAL ordinary_app=0 sandbox_app=1 sid_equal=1 integrity_rid=4096 capabilities=0 read=5 write=5 network=10013 ordinary_accepted=1 accepted=0 fixture_unchanged=1",
            "observable sandbox profile/staging cleanup": "SANDBOX_CLEANUP profile_deleted=1 staging_removed=1",
            "complete sandbox smoke": "SANDBOX_SMOKE_OK token=verified render=equal denials=verified cleanup=ok",
        }
        for name, marker in required_sandbox_markers.items():
            require_marker(name, sandbox, marker)
    else:
        for name in (
            "ordinary process smoke",
            "AppContainer renderer and denial smoke",
            "actual helper loopback WSAEACCES 10013",
        ):
            assertion(name, False, "not run after application build failure")

    if helper_build is not None and helper_build["returncode"] == 0:
        lifetime = run_command(
            "AppContainer Job/handle/cancellation lifetime",
            [str(helper), "sandbox-lifetime"],
            REPO,
            timeout=90,
            minimal_environment=True,
        )
        assertion("lifetime exit", lifetime["returncode"] == 0, f"returncode={lifetime['returncode']}")
        lifetime_markers = {
            "wrong-path fail closed": "SANDBOX_LIFETIME_STAGE wrong_path=denied",
            "Job close kills real sandbox child": "SANDBOX_LIFETIME_STAGE job_close=1",
            "two-process descendant positive control": "SANDBOX_LIFETIME_STAGE quota=2 result=STARTED_AND_REAPED",
            "one-process quota 1816": "SANDBOX_LIFETIME_STAGE quota=1 result=DENIED:1816",
            "included handle marker positive control": "SANDBOX_LIFETIME_STAGE handle_include=true result=INHERITED",
            "excluded strict-handle observation": "SANDBOX_LIFETIME_STAGE handle_include=false result=0xc0000008",
            "real pipe cancellation 995": "SANDBOX_LIFETIME_STAGE cancellation=995",
            "lifetime summary": "SANDBOX_LIFETIME job_close=1 quota_control=started quota_denied=1816 include_control=1 unrelated_handle=0xc0000008 cancellation=995 wrong_path=denied",
            "lifetime reaping and cleanup": "SANDBOX_LIFETIME_OK reaped=1 cleanup=1",
        }
        for name, marker in lifetime_markers.items():
            require_marker(name, lifetime, marker)
        parent_death = run_command(
            "AppContainer parent death and owned recovery",
            [str(helper), "sandbox-parent-death"],
            REPO,
            timeout=90,
            minimal_environment=True,
        )
        assertion("parent-death exit", parent_death["returncode"] == 0, f"returncode={parent_death['returncode']}")
        require_marker(
            "parent death kills child and recovers owned profile/staging",
            parent_death,
            "SANDBOX_PARENT_DEATH killed=1 child_reaped=1 profile_deleted=1 staging_removed=1",
        )
    else:
        assertion("AppContainer lifetime controls", False, "not run after helper build failure")
        assertion("AppContainer parent-death control", False, "not run after helper build failure")

    remaining = list(runs.iterdir()) if runs.exists() else []
    assertion(
        "all generated staging removed",
        not remaining,
        f"remaining={len(remaining)}",
    )
    passed = all(item["passed"] for item in ASSERTIONS)
    report = {
        "schema": "opensequoia-s03c-acceptance-v1",
        "generated_utc": datetime.now(timezone.utc).isoformat(),
        "revision": revision,
        "integration": provenance,
        "passed": passed,
        "assertions": ASSERTIONS,
        "commands": [
            {key: value for key, value in command.items() if key not in ("stdout", "stderr", "combined")}
            | {
                "stdout_sha256": hashlib.sha256(command["stdout"].encode("utf-8", errors="replace")).hexdigest(),
                "stderr_sha256": hashlib.sha256(command["stderr"].encode("utf-8", errors="replace")).hexdigest(),
            }
            for command in COMMANDS
        ],
        "unmet": [item["name"] for item in ASSERTIONS if not item["passed"]],
        "limits": [
            "Development probe only; no production sandbox, site/origin isolation, authenticated IPC, TLS or remote-browsing claim.",
            "Only the current Windows x64 host was exercised; no external network, personal profile, firewall change or loopback exemption was used.",
            "A timeout or SO_ERROR=0 is not WSAEACCES 10013 and cannot satisfy the network gate.",
        ],
    }
    LOG_LINES.append(f"revision={revision}")
    LOG_LINES.append(f"integration={json.dumps(provenance, sort_keys=True)}")
    for command in COMMANDS:
        LOG_LINES.append(
            f"COMMAND {command['label']} returncode={command['returncode']} "
            f"timed_out={command['timed_out']} argv={json.dumps(command['arguments'])}"
        )
        for stream in ("stdout", "stderr"):
            text, truncated = bounded(command[stream])
            LOG_LINES.append(f"{stream.upper()} truncated={truncated}\n{text}")
    for item in ASSERTIONS:
        LOG_LINES.append(
            f"{'PASS' if item['passed'] else 'FAIL'} {item['name']}: {item['evidence']}"
        )
    LOG_LINES.append(f"passed={passed} unmet={json.dumps(report['unmet'])}")
    JSON_PATH.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    LOG_PATH.write_text("\n".join(LOG_LINES) + "\n", encoding="utf-8")
    if passed:
        print("S03C_PROBE_OK")
        return 0
    print(f"S03C_PROBE_FAILED unmet={len(report['unmet'])}")
    for name in report["unmet"]:
        print(f"UNMET {name}")
    return 1


if __name__ == "__main__":
    try:
        sys.exit(main())
    except Exception as error:
        # Preserve a bounded failure record instead of emitting a success token.
        failure = {
            "schema": "opensequoia-s03c-acceptance-v1",
            "passed": False,
            "harness_error": f"{type(error).__name__}: {error}",
            "assertions": ASSERTIONS,
        }
        JSON_PATH.write_text(json.dumps(failure, indent=2) + "\n", encoding="utf-8")
        LOG_PATH.write_text(f"HARNESS_ERROR {type(error).__name__}: {error}\n", encoding="utf-8")
        print(f"S03C_PROBE_HARNESS_ERROR {type(error).__name__}: {error}")
        sys.exit(2)
