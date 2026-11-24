"""Measure a clean-profile Brave process tree; no first-paint or parity claim."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import statistics
import subprocess
import tempfile
import time

import psutil


def sample(root, known):
    processes = [root] + root.children(recursive=True)
    rss = private = cpu = 0
    count = 0
    for process in processes:
        try:
            born = process.create_time()
            known[process.pid] = born
            info = process.memory_info()
            rss += info.rss
            private += info.private
            times = process.cpu_times()
            cpu += times.user + times.system
            count += 1
        except (psutil.NoSuchProcess, psutil.AccessDenied):
            continue
    return {"processes": count, "working_set_bytes": rss,
            "private_commit_bytes": private, "cpu_seconds": cpu}


def run(executable, fixture, profile_root, seconds):
    profile = Path(tempfile.mkdtemp(prefix="brave-idle-", dir=profile_root)).resolve()
    if os.path.commonpath([profile, profile_root]) != str(profile_root):
        raise RuntimeError("profile escaped dedicated measurement directory")
    startup = subprocess.STARTUPINFO()
    startup.dwFlags |= subprocess.STARTF_USESHOWWINDOW
    startup.wShowWindow = 0
    known = {}
    child = None
    try:
        before = time.perf_counter()
        child = subprocess.Popen([str(executable), f"--user-data-dir={profile}",
                                  "--no-first-run", "--no-default-browser-check", fixture.as_uri()],
                                 startupinfo=startup, stdout=subprocess.DEVNULL,
                                 stderr=subprocess.DEVNULL)
        spawn_ms = (time.perf_counter() - before) * 1000
        root = psutil.Process(child.pid)
        rows = []
        deadline = time.perf_counter() + seconds
        while time.perf_counter() < deadline:
            if child.poll() is not None:
                raise RuntimeError(f"isolated Brave root exited early: {child.returncode}")
            rows.append(sample(root, known))
            time.sleep(0.25)
        if not rows or rows[-1]["processes"] < 2:
            raise RuntimeError("no isolated browser descendant tree observed")
        return {"process_spawn_ms": spawn_ms, "idle_after_seconds": seconds,
                "final": rows[-1], "sampled_peak_private_bytes": max(r["private_commit_bytes"] for r in rows),
                "sampled_peak_working_set_bytes": max(r["working_set_bytes"] for r in rows),
                "samples": rows}
    finally:
        # Only launch-owned PIDs with unchanged creation time are eligible for cleanup.
        for pid, born in reversed(list(known.items())):
            try:
                process = psutil.Process(pid)
                if process.create_time() == born:
                    process.kill()
            except (psutil.NoSuchProcess, psutil.AccessDenied):
                pass
        if child is not None:
            try:
                child.wait(timeout=10)
            except subprocess.TimeoutExpired:
                child.kill()
                child.wait(timeout=10)
        for attempt in range(20):
            try:
                if profile.is_relative_to(profile_root) and profile.name.startswith("brave-idle-"):
                    shutil.rmtree(profile)
                break
            except PermissionError:
                time.sleep(0.25)
        else:
            raise RuntimeError("measurement profile cleanup failed; retained under ignored raw directory")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--executable", type=Path, required=True)
    parser.add_argument("--fixture", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--runs", type=int, default=7)
    parser.add_argument("--seconds", type=float, default=5.0)
    args = parser.parse_args()
    if not 1 <= args.runs <= 30 or not 1 <= args.seconds <= 30:
        parser.error("runs and idle duration outside bounded range")
    workspace = Path(__file__).resolve().parents[1]
    profile_root = (workspace / "evidence/artifacts/raw/profiles").resolve()
    profile_root.mkdir(parents=True, exist_ok=True)
    executable, fixture = args.executable.resolve(strict=True), args.fixture.resolve(strict=True)
    rows = []
    for index in range(args.runs):
        rows.append(run(executable, fixture, profile_root, args.seconds))
        print(f"completed isolated-profile run {index + 1}/{args.runs}", flush=True)
    report = {"schema": 1, "selection_date": "2026-11-22", "kind": "warm clean-profile idle snapshot",
              "executable": str(executable), "executable_sha256": hashlib.sha256(executable.read_bytes()).hexdigest(),
              "fixture_sha256": hashlib.sha256(fixture.read_bytes()).hexdigest(), "psutil_version": psutil.__version__,
              "flags": ["--user-data-dir=<new temporary profile>", "--no-first-run", "--no-default-browser-check", "<fixture file URI>"],
              "limitations": ["Hidden window; not an interaction/frame or first-paint measure", "No controlled cold-cache run",
                               "New profiles each run; no personal data", "Sampling may miss transient processes/peaks",
                               "No equivalent OpenSequoia comparison at text-only milestone", "Background startup network traffic not disabled",
                               "Uncontrolled host background workload/power plan; exploratory measurements only"],
              "runs": rows,
              "median_final_private_bytes": statistics.median(r["final"]["private_commit_bytes"] for r in rows),
              "median_final_working_set_bytes": statistics.median(r["final"]["working_set_bytes"] for r in rows)}
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, indent=2), encoding="utf-8")
    print(json.dumps({k: v for k, v in report.items() if k.startswith("median_")}), flush=True)


if __name__ == "__main__":
    main()
