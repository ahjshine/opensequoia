# /// script
# requires-python = ">=3.11"
# dependencies = ["psutil==7.1.2"]
# ///
"""Sample the local development GUI process tree; no browser-parity speed claim."""

import argparse
import hashlib
import json
import platform
import statistics
import subprocess
import sys
import time
from datetime import UTC, datetime
from pathlib import Path

import psutil
from measure_idle import sample


def fingerprint(path):
    return {
        "path": str(path),
        "bytes": path.stat().st_size,
        "sha256": hashlib.sha256(path.read_bytes()).hexdigest(),
    }


def environment():
    power = subprocess.run(
        ["powercfg", "/getactivescheme"],
        capture_output=True,
        text=True,
        timeout=10,
        check=False,
    )
    version = sys.getwindowsversion()
    return {
        "os": platform.platform(),
        "windows_build": version.build,
        "architecture": platform.machine(),
        "processor": platform.processor(),
        "python": platform.python_version(),
        "psutil": psutil.__version__,
        "logical_cpus": psutil.cpu_count(),
        "physical_cpus": psutil.cpu_count(logical=False),
        "installed_memory_bytes": psutil.virtual_memory().total,
        "available_memory_bytes": psutil.virtual_memory().available,
        "power_scheme": power.stdout.strip(),
        "power_scheme_exit_code": power.returncode,
    }


def cleanup(child, known):
    """Only Popen-owned root and discovered descendants with matching birth times."""
    remaining = []
    errors = []
    for pid, born in reversed(list(known.items())):
        try:
            process = psutil.Process(pid)
            if process.create_time() == born:
                process.kill()
                remaining.append(process)
        except psutil.NoSuchProcess:
            continue
        except psutil.AccessDenied:
            errors.append({"pid": pid, "error": "AccessDenied"})
    # Popen holds the launch-owned process handle on Windows; no PID/name lookup.
    if child is not None:
        try:
            child.wait(timeout=5)
        except subprocess.TimeoutExpired:
            child.kill()
            child.wait(timeout=5)
    _, alive = psutil.wait_procs(remaining, timeout=5)
    if errors or alive:
        raise RuntimeError(
            f"owned process cleanup failed: errors={errors}, alive={[p.pid for p in alive]}"
        )


def run(executable, fixture, seconds, interval):
    startup = subprocess.STARTUPINFO()
    startup.dwFlags |= subprocess.STARTF_USESHOWWINDOW
    startup.wShowWindow = 0
    child = None
    known = {}
    rows = []
    try:
        before = time.perf_counter()
        child = subprocess.Popen(
            [str(executable), str(fixture)],
            startupinfo=startup,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
        spawn_ms = (time.perf_counter() - before) * 1000
        root = psutil.Process(child.pid)
        root_born = root.create_time()
        known[root.pid] = root_born
        deadline = before + seconds
        next_sample = time.perf_counter()
        while time.perf_counter() < deadline:
            if child.poll() is not None:
                raise RuntimeError(f"owned GUI root exited early: {child.returncode}")
            row = sample(root, known)
            # The shared helper may omit vanished/inaccessible descendants. The
            # launch-owned root must remain observable for a valid idle sample.
            root.memory_info()
            if row["processes"] < 1 or known.get(root.pid) != root_born:
                raise RuntimeError("owned GUI root identity was not sampled")
            row["elapsed_ms"] = (time.perf_counter() - before) * 1000
            rows.append(row)
            next_sample += interval
            time.sleep(max(0, min(next_sample, deadline) - time.perf_counter()))
        if not rows or child.poll() is not None:
            raise RuntimeError("no live owned GUI samples")
        # Capture any still-live descendants immediately before cleanup.
        for process in root.children(recursive=True):
            try:
                known[process.pid] = process.create_time()
            except psutil.NoSuchProcess:
                continue
        return {
            "process_spawn_ms": spawn_ms,
            "sample_duration_seconds": seconds,
            "sample_interval_seconds": interval,
            "final": rows[-1],
            "observed_process_identities": [
                {"pid": pid, "created_unix_seconds": born}
                for pid, born in known.items()
            ],
            "maximum_sampled_process_count": max(r["processes"] for r in rows),
            "sampled_peak_private_bytes": max(r["private_commit_bytes"] for r in rows),
            "sampled_peak_working_set_bytes": max(r["working_set_bytes"] for r in rows),
            "samples": rows,
        }
    finally:
        cleanup(child, known)


def summary(runs):
    values = {}
    for name, field in [
        ("final_private_bytes", "private_commit_bytes"),
        ("final_working_set_bytes", "working_set_bytes"),
        ("final_process_count", "processes"),
    ]:
        data = [row["final"][field] for row in runs]
        values[name] = {
            "median": statistics.median(data),
            "min": min(data),
            "max": max(data),
        }
    for field in ["sampled_peak_private_bytes", "sampled_peak_working_set_bytes"]:
        data = [row[field] for row in runs]
        values[field] = {
            "median": statistics.median(data),
            "min": min(data),
            "max": max(data),
        }
    return values


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--executable", type=Path, required=True)
    parser.add_argument("--fixture", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument(
        "--source-revision",
        required=True,
        help="Historical application source checkpoint for the supplied EXE",
    )
    parser.add_argument("--expected-sha256", required=True)
    parser.add_argument("--runs", type=int, default=7)
    parser.add_argument("--seconds", type=float, default=5)
    parser.add_argument("--interval", type=float, default=0.25)
    args = parser.parse_args()
    if sys.platform != "win32":
        parser.error("this sampler requires Windows")
    if (
        not 1 <= args.runs <= 30
        or not 1 <= args.seconds <= 30
        or not 0.05 <= args.interval <= 1
    ):
        parser.error("runs, duration or interval outside bounded range")
    executable = args.executable.resolve(strict=True)
    fixture = args.fixture.resolve(strict=True)
    before = {
        "executable": fingerprint(executable),
        "fixture": fingerprint(fixture),
        "environment": environment(),
    }
    if before["executable"]["sha256"] != args.expected_sha256.lower():
        parser.error("executable SHA256 does not match the approved measurement target")
    if not fixture.is_file() or fixture.stat().st_size > 262144:
        parser.error("fixture must be a bounded trusted regular HTML file")
    started = datetime.now(UTC).isoformat()
    rows = []
    for index in range(args.runs):
        rows.append(run(executable, fixture, args.seconds, args.interval))
        print(f"completed local GUI idle run {index + 1}/{args.runs}", flush=True)
    after = {
        "executable": fingerprint(executable),
        "fixture": fingerprint(fixture),
        "environment": environment(),
    }
    if any(before[key] != after[key] for key in ["executable", "fixture"]):
        raise RuntimeError("executable or fixture changed during measurements")
    report = {
        "schema": 1,
        "task": "M02",
        "started_utc": started,
        "finished_utc": datetime.now(UTC).isoformat(),
        "kind": "warm local-development GUI exploratory idle snapshots",
        "source_revision": args.source_revision,
        "sampler_sources": [
            fingerprint(Path(__file__).resolve()),
            fingerprint(Path(__file__).with_name("measure_idle.py").resolve()),
        ],
        "command": [str(Path(__file__).resolve()), *sys.argv[1:]],
        "before": before,
        "after": after,
        "runs": rows,
        "summary": summary(rows),
        "window_policy": "STARTF_USESHOWWINDOW with SW_HIDE requested; no interaction",
        "limitations": [
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
        ],
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, indent=2), encoding="utf-8")
    print(json.dumps(report["summary"]), flush=True)


if __name__ == "__main__":
    main()
