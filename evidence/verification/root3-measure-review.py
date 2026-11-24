"""Independently reconcile M02 raw observations, source fingerprints and statistics."""
import hashlib
import json
from pathlib import Path
import statistics

workspace = Path(__file__).resolve().parents[2]
report = json.loads((workspace / "evidence/benchmarks/app2-idle.json").read_text(encoding="utf-8"))
checks = []
for name in ["executable", "fixture"]:
    item = report["before"][name]
    checks.append(report["after"][name] == item)
    checks.append(hashlib.sha256(Path(item["path"]).read_bytes()).hexdigest() == item["sha256"])
for source in report["sampler_sources"]:
    checks.append(hashlib.sha256(Path(source["path"]).read_bytes()).hexdigest() == source["sha256"])
runs = report["runs"]
checks.extend([len(runs) == 7, all(len(run["samples"]) == 20 for run in runs)])
for run in runs:
    checks.append(run["final"] == run["samples"][-1])
    for field, sample_field in [("sampled_peak_private_bytes", "private_commit_bytes"),
                                ("sampled_peak_working_set_bytes", "working_set_bytes")]:
        checks.append(run[field] == max(row[sample_field] for row in run["samples"]))
for name, field in [("final_private_bytes", "private_commit_bytes"),
                    ("final_working_set_bytes", "working_set_bytes"),
                    ("final_process_count", "processes")]:
    values = [run["final"][field] for run in runs]
    checks.append(report["summary"][name] == {
        "median": statistics.median(values), "min": min(values), "max": max(values)})
result = {"task": "M02", "reviewer": "root (separate from sampler author)",
          "checks": len(checks), "all_pass": all(checks), "actual_measurement_rerun": False,
          "sampled_process_counts": sorted({s["processes"] for r in runs for s in r["samples"]}),
          "limits": ["Only recorded observations/statistics reconciled",
                     "No sampled renderer child; true peak/paint-readiness unknown",
                     "No equivalent Brave gain or production budget accepted"]}
(workspace / "evidence/verification/root3-measure-review.json").write_text(json.dumps(result, indent=2), encoding="utf-8")
print(json.dumps(result))
raise SystemExit(not result["all_pass"])
