"""Recorded Craft Anything 0.6.1 CLI calls for the I03a EXE investigation."""
import json
from pathlib import Path
import subprocess
import sys

WORKSPACE = Path(__file__).resolve().parents[2]
CLI = Path("C:/Users/deepa/AppData/Local/Programs/CraftAnything/v0.6.1/craft-anything-cli.exe")
PROJECT = WORKSPACE / "evidence/artifacts/sandbox4-opensequoia.cany"


def call(name, tool, params):
    params = dict(params)
    target = params.pop("_target", None)
    command = [str(CLI), "--project", str(PROJECT), "--deep", tool]
    if target:
        command.append(str(target))
    command.extend(["--json", json.dumps(params)])
    result = subprocess.run(command, cwd=WORKSPACE, capture_output=True, text=True, encoding="utf-8")
    record = {
        "tool": tool,
        "params": params,
        "exit_code": result.returncode,
        "stderr": result.stderr,
        "stdout": result.stdout,
    }
    destination = WORKSPACE / f"evidence/artifacts/sandbox4-craft-{name}.json"
    destination.write_text(json.dumps(record, indent=2), encoding="utf-8")
    if result.returncode:
        raise RuntimeError(f"{tool} failed: {result.stderr or result.stdout}")
    data = json.loads(result.stdout)
    if "error" in data:
        raise RuntimeError(f"{tool}: {data['error']}")
    print(json.dumps({"name": name, "tool": tool, "output": str(destination)}))
    return data


if __name__ == "__main__":
    call(sys.argv[1], sys.argv[2], json.loads(sys.argv[3]))