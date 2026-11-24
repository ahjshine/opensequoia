"""Bind the existing GUI verification inputs to the accepted Git source snapshot."""
import hashlib
import json
from pathlib import Path
import subprocess

workspace = Path(__file__).resolve().parents[2]
commit = subprocess.check_output(
    ["git", "rev-parse", "83451aa"], cwd=workspace, text=True).strip()
verification = json.loads((workspace / "evidence/verification/root2-gui-verification.json").read_text(encoding="utf-8-sig"))
rows = []
for entry in verification["input_hashes"]:
    path = entry["path"].replace("\\", "/")
    committed = subprocess.check_output(["git", "show", f"{commit}:{path}"], cwd=workspace)
    digest = hashlib.sha256(committed).hexdigest()
    rows.append({"path": path, "sha256": digest, "matches": digest == entry["sha256"].lower()})
report = {"task": "I02", "source_commit": commit,
          "verification": "evidence/verification/root2-gui-verification.json",
          "checked": len(rows), "mismatches": [r for r in rows if not r["matches"]], "inputs": rows,
          "executable_sha256": "d80778d7dbaa7f3a520b33e88ab3cb8346cce1c8c31749cba19653ede0511f3e",
          "zip_sha256": "743c6594a6e72f0dffdcfbf07537b4900659da35d018933eef39df21d871ff05",
          "artifact_evidence": "evidence/artifacts/gui2-acceptance.json",
          "limitations": ["Matches saved source inputs; not a reproducible compiler-build proof",
                          "Development source has since advanced beyond this immutable GUI package"]}
(workspace / "evidence/verification/root3-checkpoint.json").write_text(json.dumps(report, indent=2), encoding="utf-8")
print(json.dumps({k: report[k] for k in ["source_commit", "checked", "mismatches"]}))
raise SystemExit(bool(report["mismatches"]))
