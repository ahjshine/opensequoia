"""Independent M02 evidence oracle; never launches the measured GUI workload."""

# Value: protects=M02 raw arithmetic, source identity, baseline integrity, and owned cleanup
# fails_when=a sample, digest, summary, PID birth identity, rejection, or Git baseline changes
# why_new=focused checks omit all-sample, environment, hash, and wrong-hash reconciliation
# seam=none
# no_claim=no GUI rerun, paint/readiness, renderer/true peak, Brave equivalence, or budgets

import argparse
import copy
import hashlib
import importlib.util
import io
import json
import math
import os
import subprocess
import sys
from collections import Counter
from contextlib import redirect_stderr
from datetime import datetime
from pathlib import Path
from unittest import mock

import psutil


WORKSPACE = Path(__file__).resolve().parents[2]
LEGACY_WORKSPACE = WORKSPACE.parent.parent / "opensequoia"
REPORT_PATH = WORKSPACE / "evidence/benchmarks/app2-idle.json"
APP_SAMPLER_PATH = WORKSPACE / "scripts/measure_app_idle.py"
LEGACY_APP_SAMPLER_PATH = LEGACY_WORKSPACE / "scripts/measure_app_idle.py"
EXECUTABLE_PATH = LEGACY_WORKSPACE / "target/gui/release/opensequoia.exe"
FIXTURE_PATH = LEGACY_WORKSPACE / "fixtures/welcome.html"
BRAVE_SAMPLER_PATH = LEGACY_WORKSPACE / "scripts/measure_idle.py"
BRAVE_REPORT_PATH = LEGACY_WORKSPACE / "evidence/benchmarks/brave-idle.json"
BUILD_RECORD_PATH = LEGACY_WORKSPACE / "evidence/artifacts/opensequoia-0.2.0-build.json"
HASH_REJECTION_LOG = LEGACY_WORKSPACE / "evidence/verification/measure2-hash-rejection.log"
BASELINE_PATHS = (BRAVE_SAMPLER_PATH, BRAVE_REPORT_PATH)
EXPECTED_REPORT_SHA256 = "ac5e2799ab5a8c91a895cccaeb81a2c083a5f69a101a49a8580fb5b2a16c004d"
EXPECTED_SOURCE_REVISION = "83451aa1a93f7c85b730dd30940975bd7abf6aaa"
EXPECTED_EXECUTABLE_SHA256 = "d80778d7dbaa7f3a520b33e88ab3cb8346cce1c8c31749cba19653ede0511f3e"
EXPECTED_FIXTURE_SHA256 = "68cedd95f6f8125e334f748f1090cfb29f27fa7affd37e86214a90469c2294ea"
EXPECTED_ARGUMENTS = [
    "--executable",
    "target/gui/release/opensequoia.exe",
    "--fixture",
    "fixtures/welcome.html",
    "--output",
    "evidence/benchmarks/app2-idle.json",
    "--source-revision",
    EXPECTED_SOURCE_REVISION,
    "--expected-sha256",
    EXPECTED_EXECUTABLE_SHA256,
    "--runs",
    "7",
    "--seconds",
    "5",
    "--interval",
    "0.25",
]
REQUIRED_ENVIRONMENT_FIELDS = {
    "os": str,
    "windows_build": int,
    "architecture": str,
    "processor": str,
    "python": str,
    "psutil": str,
    "logical_cpus": int,
    "physical_cpus": int,
    "installed_memory_bytes": int,
    "available_memory_bytes": int,
    "power_scheme": str,
    "power_scheme_exit_code": int,
}
STABLE_ENVIRONMENT_FIELDS = tuple(
    field for field in REQUIRED_ENVIRONMENT_FIELDS if field != "available_memory_bytes"
)
REQUIRED_LIMITATIONS = (
    "Immutable0.2 local application EXE; current development source may have advanced",
    "No profiles implemented or used; fixed trusted local fixture only",
    "Hidden-window exploratory samples; no first-paint/readiness/interaction verdict",
    "No controlled cold-cache run; final sample near five seconds is idle observation",
    "Transient children and memory/CPU peaks between samples may be missed",
    "Shared sampler omits vanished or access-denied processes; root accessibility is checked",
    "Root remains after one-shot renderer exits; one sampled process is valid",
    "Working-set sum may count shared pages more than once",
    "CPU seconds sum applies only to extant sampled processes, not total exited-child CPU",
    "Host background workload and available RAM/power state are uncontrolled",
    "Retained full-browser functionality is incomplete; no equivalent Brave advantage or budgets",
    "Startup Popen duration is process creation only; not startup-to-paint or navigation speed",
)


class Audit:
    def __init__(self):
        self.checks = []

    def require(self, category, description, condition, observed=None):
        record = {"category": category, "description": description, "passed": bool(condition)}
        if observed is not None:
            record["observed"] = observed
        self.checks.append(record)
        if not condition:
            raise AssertionError(f"{category}: {description}; observed={observed!r}")

    @property
    def categories(self):
        counts = Counter(check["category"] for check in self.checks)
        return dict(sorted(counts.items()))


def fingerprint(path):
    data = path.read_bytes()
    return {"bytes": len(data), "sha256": hashlib.sha256(data).hexdigest()}


def median(values):
    """Recompute median independently rather than calling the sampler's summary."""
    ordered = sorted(values)
    if not ordered:
        raise ValueError("median requires at least one value")
    middle = len(ordered) // 2
    if len(ordered) % 2:
        return ordered[middle]
    return (ordered[middle - 1] + ordered[middle]) / 2


def distribution(values):
    return {"median": median(values), "min": min(values), "max": max(values)}


def recompute_summary(runs):
    result = {}
    for name, field in (
        ("final_private_bytes", "private_commit_bytes"),
        ("final_working_set_bytes", "working_set_bytes"),
        ("final_process_count", "processes"),
    ):
        result[name] = distribution([run["final"][field] for run in runs])
    for field in ("sampled_peak_private_bytes", "sampled_peak_working_set_bytes"):
        result[field] = distribution([run[field] for run in runs])
    return result


def run_git(arguments, *, cwd=WORKSPACE):
    return subprocess.run(
        ["git", *arguments],
        cwd=cwd,
        capture_output=True,
        text=True,
        timeout=20,
        check=False,
    )


def relative(path):
    if path.is_relative_to(WORKSPACE):
        return path.relative_to(WORKSPACE).as_posix()
    return path.relative_to(LEGACY_WORKSPACE).as_posix()


def is_number(value):
    return type(value) in (int, float) and math.isfinite(value)


def is_nonnegative_integer(value):
    return type(value) is int and value >= 0


def audit_report(audit, report):
    audit.require("raw_metadata", "schema is version 1", report.get("schema") == 1)
    audit.require("raw_metadata", "task is M02", report.get("task") == "M02")
    audit.require(
        "raw_metadata",
        "measurement kind remains exploratory warm local GUI idle",
        report.get("kind") == "warm local-development GUI exploratory idle snapshots",
    )
    audit.require(
        "source_identity",
        "historical application source checkpoint is exact",
        report.get("source_revision") == EXPECTED_SOURCE_REVISION,
    )
    revision = run_git(["rev-parse", "--verify", f"{EXPECTED_SOURCE_REVISION}^{{commit}}"])
    audit.require(
        "source_identity",
        "historical source checkpoint resolves to a Git commit",
        revision.returncode == 0 and revision.stdout.strip() == EXPECTED_SOURCE_REVISION,
        {"exit_code": revision.returncode, "resolved": revision.stdout.strip()},
    )
    audit.require(
        "raw_metadata",
        "recorded command names the legacy app sampler",
        Path(report["command"][0]).resolve() == LEGACY_APP_SAMPLER_PATH.resolve(),
    )
    audit.require(
        "raw_metadata",
        "recorded arguments preserve the immutable seven-run workload",
        report["command"][1:] == EXPECTED_ARGUMENTS,
    )
    audit.require(
        "raw_metadata",
        "hidden-window policy is explicit",
        report.get("window_policy")
        == "STARTF_USESHOWWINDOW with SW_HIDE requested; no interaction",
    )

    current_fingerprints = {}
    for name, expected_path, expected_hash in (
        ("executable", EXECUTABLE_PATH, EXPECTED_EXECUTABLE_SHA256),
        ("fixture", FIXTURE_PATH, EXPECTED_FIXTURE_SHA256),
    ):
        before = report["before"][name]
        after = report["after"][name]
        audit.require(
            "source_identity", f"{name} fingerprint is unchanged before/after", before == after
        )
        audit.require(
            "source_identity",
            f"{name} recorded path is the reviewed current path",
            Path(before["path"]).resolve() == expected_path.resolve(),
        )
        current = fingerprint(expected_path)
        current_fingerprints[name] = current
        audit.require(
            "source_identity",
            f"current {name} bytes and SHA256 match both recorded fingerprints",
            current == {"bytes": before["bytes"], "sha256": before["sha256"]},
            current,
        )
        audit.require(
            "source_identity",
            f"{name} SHA256 matches the approved immutable identity",
            current["sha256"] == expected_hash,
        )

    build_record = json.loads(BUILD_RECORD_PATH.read_text(encoding="utf-8"))
    audit.require(
        "source_identity",
        "GUI build record binds the same executable bytes and SHA256",
        build_record["executable_bytes"] == current_fingerprints["executable"]["bytes"]
        and build_record["executable_sha256"] == current_fingerprints["executable"]["sha256"],
    )
    audit.require(
        "source_identity",
        "GUI build record names the measured executable",
        Path(build_record["build_executable"]).resolve() == EXECUTABLE_PATH.resolve(),
    )

    expected_sources = (
        (LEGACY_APP_SAMPLER_PATH, APP_SAMPLER_PATH),
        (BRAVE_SAMPLER_PATH, BRAVE_SAMPLER_PATH),
    )
    audit.require(
        "source_identity",
        "exactly two sampler sources are recorded",
        len(report["sampler_sources"]) == len(expected_sources),
    )
    source_fingerprints = {}
    for recorded, (recorded_path, current_path) in zip(
        report["sampler_sources"], expected_sources, strict=True
    ):
        audit.require(
            "source_identity",
            f"sampler source path is {relative(recorded_path)}",
            Path(recorded["path"]).resolve() == recorded_path.resolve(),
        )
        expected_fingerprint = {"bytes": recorded["bytes"], "sha256": recorded["sha256"]}
        legacy_current = fingerprint(recorded_path)
        current = fingerprint(current_path)
        source_fingerprints[relative(current_path)] = current
        audit.require(
            "source_identity",
            f"legacy and reviewed {relative(current_path)} fingerprints match raw evidence",
            legacy_current == expected_fingerprint and current == expected_fingerprint,
            {"legacy": legacy_current, "reviewed": current},
        )

    started = datetime.fromisoformat(report["started_utc"])
    finished = datetime.fromisoformat(report["finished_utc"])
    audit.require("raw_metadata", "measurement timestamps are ordered", started < finished)
    runs = report["runs"]
    audit.require("raw_structure", "exactly seven runs are present", len(runs) == 7, len(runs))
    audit.require(
        "raw_structure",
        "summary has exactly the five published distributions",
        set(report["summary"])
        == {
            "final_private_bytes",
            "final_working_set_bytes",
            "final_process_count",
            "sampled_peak_private_bytes",
            "sampled_peak_working_set_bytes",
        },
    )

    all_identities = []
    sampled_process_counts = set()
    for index, run in enumerate(runs, start=1):
        prefix = f"run {index}"
        samples = run["samples"]
        audit.require(
            "raw_structure", f"{prefix} has twenty samples", len(samples) == 20, len(samples)
        )
        audit.require(
            "raw_structure",
            f"{prefix} retains five-second/250ms nominal settings",
            run["sample_duration_seconds"] == 5.0 and run["sample_interval_seconds"] == 0.25,
        )
        audit.require(
            "raw_structure",
            f"{prefix} process-spawn field is a finite nonnegative observation",
            is_number(run["process_spawn_ms"]) and run["process_spawn_ms"] >= 0,
        )
        required_sample_fields = {
            "processes",
            "working_set_bytes",
            "private_commit_bytes",
            "cpu_seconds",
            "elapsed_ms",
        }
        audit.require(
            "raw_structure",
            f"{prefix} sample fields are complete",
            all(set(sample) == required_sample_fields for sample in samples),
        )
        audit.require(
            "raw_structure",
            f"{prefix} memory fields are nonnegative integers",
            all(
                is_nonnegative_integer(sample[field])
                for sample in samples
                for field in ("working_set_bytes", "private_commit_bytes")
            ),
        )
        audit.require(
            "raw_structure",
            f"{prefix} process counts are positive integers",
            all(type(sample["processes"]) is int and sample["processes"] >= 1 for sample in samples),
        )
        audit.require(
            "raw_structure",
            f"{prefix} CPU and elapsed fields are finite and nonnegative",
            all(
                is_number(sample[field]) and sample[field] >= 0
                for sample in samples
                for field in ("cpu_seconds", "elapsed_ms")
            ),
        )
        audit.require(
            "raw_structure",
            f"{prefix} elapsed observations increase and stay within the run deadline",
            all(0 < sample["elapsed_ms"] <= 5000 for sample in samples)
            and all(
                earlier["elapsed_ms"] < later["elapsed_ms"]
                for earlier, later in zip(samples, samples[1:])
            ),
        )
        audit.require(
            "raw_structure",
            f"{prefix} extant-root CPU observation is nondecreasing",
            all(
                earlier["cpu_seconds"] <= later["cpu_seconds"]
                for earlier, later in zip(samples, samples[1:])
            ),
        )
        audit.require(
            "derived_statistics", f"{prefix} final object is the last raw sample", run["final"] == samples[-1]
        )
        audit.require(
            "derived_statistics",
            f"{prefix} sampled private peak is recomputed from all samples",
            run["sampled_peak_private_bytes"]
            == max(sample["private_commit_bytes"] for sample in samples),
        )
        audit.require(
            "derived_statistics",
            f"{prefix} sampled working-set peak is recomputed from all samples",
            run["sampled_peak_working_set_bytes"]
            == max(sample["working_set_bytes"] for sample in samples),
        )
        audit.require(
            "derived_statistics",
            f"{prefix} maximum sampled process count is recomputed from all samples",
            run["maximum_sampled_process_count"]
            == max(sample["processes"] for sample in samples),
        )
        sampled_process_counts.update(sample["processes"] for sample in samples)
        identities = run["observed_process_identities"]
        audit.require(
            "raw_structure",
            f"{prefix} records only its launch-owned root identity",
            len(identities) == 1
            and type(identities[0]["pid"]) is int
            and identities[0]["pid"] > 0
            and is_number(identities[0]["created_unix_seconds"])
            and identities[0]["created_unix_seconds"] > 0,
        )
        identity = identities[0]
        audit.require(
            "raw_structure",
            f"{prefix} root creation time lies within the report interval",
            started.timestamp() <= identity["created_unix_seconds"] <= finished.timestamp(),
        )
        all_identities.append(identity)

    audit.require(
        "raw_structure",
        "140 samples contain only one extant sampled process",
        sampled_process_counts == {1},
        sorted(sampled_process_counts),
    )
    audit.require(
        "raw_structure",
        "seven root identities have no duplicate PIDs",
        len({identity["pid"] for identity in all_identities}) == 7,
    )

    recomputed = recompute_summary(runs)
    for name, expected in recomputed.items():
        audit.require(
            "derived_statistics",
            f"{name} median/min/max exactly match independent recomputation",
            report["summary"][name] == expected,
            expected,
        )

    before_environment = report["before"]["environment"]
    after_environment = report["after"]["environment"]
    audit.require(
        "environment",
        "before/after environment field sets are complete and identical",
        set(before_environment) == set(REQUIRED_ENVIRONMENT_FIELDS)
        and set(after_environment) == set(REQUIRED_ENVIRONMENT_FIELDS),
    )
    for field, expected_type in REQUIRED_ENVIRONMENT_FIELDS.items():
        audit.require(
            "environment",
            f"environment field {field} has the recorded type in both captures",
            type(before_environment[field]) is expected_type
            and type(after_environment[field]) is expected_type,
        )
    for field in STABLE_ENVIRONMENT_FIELDS:
        audit.require(
            "environment",
            f"stable environment field {field} matches before/after",
            before_environment[field] == after_environment[field],
        )
    for label, environment in (("before", before_environment), ("after", after_environment)):
        audit.require(
            "environment",
            f"{label} CPU, memory and power values are coherent",
            environment["logical_cpus"] > 0
            and environment["physical_cpus"] > 0
            and environment["installed_memory_bytes"] > 0
            and 0 <= environment["available_memory_bytes"] <= environment["installed_memory_bytes"]
            and environment["power_scheme_exit_code"] == 0
            and bool(environment["power_scheme"]),
        )
    audit.require(
        "environment",
        "recorded sampler dependency is psutil 7.1.2",
        before_environment["psutil"] == "7.1.2",
    )
    changed_environment_fields = [
        field
        for field in REQUIRED_ENVIRONMENT_FIELDS
        if before_environment[field] != after_environment[field]
    ]
    audit.require(
        "environment",
        "only available memory changed between environment captures",
        changed_environment_fields == ["available_memory_bytes"],
        changed_environment_fields,
    )
    for limitation in REQUIRED_LIMITATIONS:
        audit.require(
            "limitations", f"limit retained: {limitation}", limitation in report["limitations"]
        )

    return {
        "recomputed_summary": recomputed,
        "sample_counts": [len(run["samples"]) for run in runs],
        "sampled_process_counts": sorted(sampled_process_counts),
        "observed_identities": all_identities,
        "final_elapsed_ms": distribution([run["final"]["elapsed_ms"] for run in runs]),
        "current_fingerprints": current_fingerprints,
        "source_fingerprints": source_fingerprints,
        "environment_changed_fields": changed_environment_fields,
    }


def audit_baseline(audit):
    baseline = {}
    for path in BASELINE_PATHS:
        name = relative(path)
        tracked = run_git(["ls-files", "--error-unmatch", "--", name], cwd=LEGACY_WORKSPACE)
        audit.require(
            "baseline_git", f"{name} is tracked", tracked.returncode == 0, tracked.returncode
        )
        worktree = run_git(["diff", "--exit-code", "--", name], cwd=LEGACY_WORKSPACE)
        audit.require(
            "baseline_git",
            f"{name} has no worktree Git diff",
            worktree.returncode == 0 and not worktree.stdout and not worktree.stderr,
            worktree.returncode,
        )
        cached = run_git(["diff", "--cached", "--exit-code", "--", name], cwd=LEGACY_WORKSPACE)
        audit.require(
            "baseline_git",
            f"{name} has no staged Git diff",
            cached.returncode == 0 and not cached.stdout and not cached.stderr,
            cached.returncode,
        )
        status = run_git(["status", "--porcelain", "--", name], cwd=LEGACY_WORKSPACE)
        audit.require(
            "baseline_git",
            f"{name} has no Git status entry",
            status.returncode == 0 and status.stdout == "",
            status.stdout,
        )
        baseline[name] = {"sha256": fingerprint(path)["sha256"], "git_diff": False}
    return baseline


def import_sampler():
    # Do not create or update __pycache__, which is outside this leaf's ownership.
    sys.dont_write_bytecode = True
    specification = importlib.util.spec_from_file_location("measure_idle", BRAVE_SAMPLER_PATH)
    if specification is None or specification.loader is None:
        raise RuntimeError("cannot load the legacy shared sampler")
    shared = importlib.util.module_from_spec(specification)
    sys.modules[specification.name] = shared
    specification.loader.exec_module(shared)
    sys.path.insert(0, str(WORKSPACE / "scripts"))
    import measure_app_idle

    return measure_app_idle


def audit_recorded_identities(audit, identities):
    states = []
    for identity in identities:
        try:
            current_birth = psutil.Process(identity["pid"]).create_time()
        except psutil.NoSuchProcess:
            states.append("not_running")
        else:
            if current_birth == identity["created_unix_seconds"]:
                states.append("same_identity_alive")
            else:
                states.append("pid_reused_different_birth")
    counts = Counter(states)
    audit.require(
        "cleanup_and_rejection",
        "no recorded launch-owned PID/birth identity remains alive",
        counts["same_identity_alive"] == 0,
        dict(counts),
    )
    return dict(counts)


def audit_wrong_hash_rejection(audit, sampler, report):
    log_text = HASH_REJECTION_LOG.read_text(encoding="utf-8")
    audit.require(
        "cleanup_and_rejection",
        "historical wrong-hash log contains the explicit rejection",
        "error: executable SHA256 does not match the approved measurement target" in log_text,
    )
    calls = []

    def forbidden_launch(*args, **kwargs):
        calls.append({"args": args, "kwargs": kwargs})
        raise AssertionError("wrong-hash negative control attempted a process/GUI launch")

    argv = [
        str(APP_SAMPLER_PATH),
        "--executable",
        str(EXECUTABLE_PATH),
        "--fixture",
        str(FIXTURE_PATH),
        "--output",
        os.devnull,
        "--source-revision",
        EXPECTED_SOURCE_REVISION,
        "--expected-sha256",
        "0" * 64,
        "--runs",
        "7",
        "--seconds",
        "5",
        "--interval",
        "0.25",
    ]
    errors = io.StringIO()
    exit_code = None
    try:
        with (
            mock.patch.object(sampler, "run", forbidden_launch),
            mock.patch.object(sampler, "environment", return_value=report["before"]["environment"]),
            mock.patch.object(sampler.subprocess, "Popen", forbidden_launch),
            mock.patch.object(sys, "argv", argv),
            redirect_stderr(errors),
        ):
            sampler.main()
    except SystemExit as error:
        exit_code = error.code
    audit.require(
        "cleanup_and_rejection",
        "wrong SHA256 exits with argparse status 2",
        exit_code == 2,
        exit_code,
    )
    audit.require(
        "cleanup_and_rejection",
        "wrong SHA256 is rejected before any run or Popen call",
        not calls,
        len(calls),
    )
    audit.require(
        "cleanup_and_rejection",
        "wrong SHA256 rejection is explicit in current-source stderr",
        "executable SHA256 does not match the approved measurement target" in errors.getvalue(),
    )
    audit.require(
        "cleanup_and_rejection",
        "wrong SHA256 negative control leaves raw evidence byte-identical",
        fingerprint(REPORT_PATH)["sha256"] == EXPECTED_REPORT_SHA256,
    )
    return {"exit_code": exit_code, "launch_calls": len(calls), "gui_workload_rerun": False}


def audit_cleanup(audit, sampler):
    child = subprocess.Popen(
        [sys.executable, "-c", "import time; time.sleep(30)"],
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )
    try:
        process = psutil.Process(child.pid)
        born = process.create_time()
        sampler.cleanup(None, {child.pid: born + 1.0})
        audit.require(
            "cleanup_and_rejection",
            "mismatched PID creation time preserves the test process",
            child.poll() is None and process.create_time() == born,
        )
        sampler.cleanup(child, {child.pid: born})
        audit.require(
            "cleanup_and_rejection",
            "matching launch-owned root is killed and reaped through its Popen handle",
            child.poll() is not None,
        )
    finally:
        if child.poll() is None:
            child.kill()
        child.wait(timeout=5)
    return {"mismatched_birth_preserved": True, "matching_owned_root_reaped": True}


def audit_summary_mutation_control(audit, report):
    mutated = copy.deepcopy(report)
    mutated["runs"][0]["final"]["private_commit_bytes"] += 4096
    audit.require(
        "derived_statistics",
        "positive control: a one-sample final-value mutation changes the recomputed distribution",
        recompute_summary(mutated["runs"]) != report["summary"],
    )


def perform_audit():
    audit = Audit()
    raw = REPORT_PATH.read_bytes()
    raw_sha256 = hashlib.sha256(raw).hexdigest()
    audit.require(
        "raw_metadata",
        "immutable raw report SHA256 is exact",
        raw_sha256 == EXPECTED_REPORT_SHA256,
        raw_sha256,
    )
    report = json.loads(raw)
    details = audit_report(audit, report)
    baseline = audit_baseline(audit)
    sampler = import_sampler()
    identity_states = audit_recorded_identities(audit, details["observed_identities"])
    wrong_hash = audit_wrong_hash_rejection(audit, sampler, report)
    cleanup = audit_cleanup(audit, sampler)
    audit_summary_mutation_control(audit, report)
    audit.require(
        "raw_metadata",
        "all dynamic checks preserve immutable raw evidence",
        fingerprint(REPORT_PATH)["sha256"] == EXPECTED_REPORT_SHA256,
    )
    return {
        "task": "M02",
        "result": "M02_ACCEPTANCE_OK",
        "checks_total": len(audit.checks),
        "checks_passed": sum(check["passed"] for check in audit.checks),
        "check_categories": audit.categories,
        "raw_sha256": raw_sha256,
        "source_revision": report["source_revision"],
        "runs": len(report["runs"]),
        "samples_per_run": details["sample_counts"],
        "total_samples": sum(details["sample_counts"]),
        "sampled_process_counts": details["sampled_process_counts"],
        "observed_renderer_children": sum(
            len(run["observed_process_identities"]) - 1 for run in report["runs"]
        ),
        "recomputed_summary": details["recomputed_summary"],
        "final_elapsed_ms": details["final_elapsed_ms"],
        "current_fingerprints": details["current_fingerprints"],
        "source_fingerprints": details["source_fingerprints"],
        "baseline": baseline,
        "environment_changed_fields": details["environment_changed_fields"],
        "recorded_identity_states": identity_states,
        "wrong_hash_negative_control": wrong_hash,
        "cleanup_positive_controls": cleanup,
        "gui_workload_rerun": False,
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--details", action="store_true", help="print a JSON audit before the success token")
    args = parser.parse_args()
    try:
        details = perform_audit()
    except Exception as error:
        print(f"M02_ACCEPTANCE_FAIL: {type(error).__name__}: {error}", file=sys.stderr)
        return 1
    if args.details:
        print(json.dumps(details, indent=2))
    print("M02_ACCEPTANCE_OK")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
