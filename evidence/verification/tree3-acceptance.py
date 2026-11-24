#!/usr/bin/env python3
"""S04a pinned tree-construction acceptance oracle.

The default invocation never updates the source pin. It independently inventories
upstream bytes, builds the real adapter, runs every applicable scripted outcome,
and rejects incomplete or operationally unhealthy reports. Engine ``fail`` is
allowed because a tree mismatch is a product result, not a harness malfunction.
"""

from __future__ import annotations

import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import tomllib
from typing import Any

ROOT = Path(__file__).resolve().parents[2]
# Run from the canonical checkout or from a sibling crew worktree; prefer the
# checkout that actually contains the pinned source corpus.
_CANDIDATE_ROOTS = (
    ROOT,
    ROOT.parent.parent / "opensequoia",
)
HTML5EVER_ROOT = next(
    (
        base / "evidence" / "components" / "html5ever"
        for base in _CANDIDATE_ROOTS
        if (base / "evidence" / "components" / "html5ever").is_dir()
    ),
    ROOT / "evidence" / "components" / "html5ever",
)
RCDOM = HTML5EVER_ROOT / "rcdom"
# Root the inventory paths at the checkout that supplies the pinned corpus.
LEGACY_ROOT = HTML5EVER_ROOT.parents[2]
SUITE = RCDOM / "wpt-tree-construction-tests"
SUITE_RELATIVE = "evidence/components/html5ever/rcdom/wpt-tree-construction-tests"
MANIFEST = ROOT / "evidence" / "verification" / "tree3-suite-hashes.json"
REPORT = ROOT / "evidence" / "verification" / "tree3-report.jsonl"
TARGET = ROOT / "target" / "tree3"
ADAPTER = TARGET / "debug" / "examples" / "tree_construction.exe"

WPT_REVISION = "31d1feb70e2d02f25ae26254caba19c8de0b56e0"
HTML5EVER_SOURCE_REVISION = "492f52a02fd07c2321e59ccff90038e26b9c90bc"
PARSER_VCS_REVISION = "d7232d746d3112f08926d169591576bd12fe2fdf"
PARSER_VERSION = "0.40.1"
EXPECTED_DAT_FILES = 59
EXPECTED_SOURCE_CASES = 1779
EXPECTED_OUTCOMES = 3519
EXPECTED_SUITE_INPUT_FILES = 63
EXPECTED_INVENTORY_FILES = 68
MAX_DAT_BYTES = 1024 * 1024
MAX_SUITE_FILE_BYTES = 2 * 1024 * 1024
MAX_REPORT_BYTES = 64 * 1024 * 1024
TIMEOUT_MS = 5000
OUTCOMES = ("pass", "fail", "unsupported", "skip", "timeout", "crash", "harness_error")
BLOCKED_OUTCOMES = ("skip", "timeout", "crash", "harness_error")
SECTION_NAMES = {
    "data",
    "document",
    "errors",
    "new-errors",
    "document-fragment",
    "script-on",
    "script-off",
}
UPSTREAM_INPUTS = (
    RCDOM / "data" / "test" / "wpt-commit.txt",
    RCDOM / "data" / "test" / "ignore",
    RCDOM / "tests" / "html-tree-builder.rs",
    RCDOM / "tests" / "foreach_html5lib_test" / "mod.rs",
    HTML5EVER_ROOT / "Cargo.toml",
)
LIMITS = (
    "tree assertions only; parser diagnostics not compared; no JS execution; "
    "source label verified by separate SHA256 inventory"
)


class AcceptanceError(RuntimeError):
    """A precise acceptance contract violation."""


def require(condition: bool, message: str) -> None:
    if not condition:
        raise AcceptanceError(message)


def comparable_path(path: Path) -> str:
    value = str(path)
    if os.name == "nt" and value.startswith("\\\\?\\UNC\\"):
        value = "\\\\" + value[8:]
    elif os.name == "nt" and value.startswith("\\\\?\\"):
        value = value[4:]
    return os.path.normcase(os.path.abspath(value))


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(65536), b""):
            digest.update(chunk)
    return digest.hexdigest()


def file_record(path: Path) -> dict[str, Any]:
    require(path.is_file() and not path.is_symlink(), f"not a regular non-symlink file: {path}")
    size = path.stat().st_size
    require(size <= MAX_SUITE_FILE_BYTES, f"suite input exceeds {MAX_SUITE_FILE_BYTES} bytes: {path}")
    return {
        "path": path.relative_to(LEGACY_ROOT).as_posix(),
        "bytes": size,
        "sha256": sha256(path),
    }


def run_command(command: list[str], timeout: int) -> subprocess.CompletedProcess[str]:
    try:
        return subprocess.run(
            command,
            cwd=ROOT,
            stdin=subprocess.DEVNULL,
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace",
            timeout=timeout,
            check=False,
        )
    except subprocess.TimeoutExpired as error:
        raise AcceptanceError(f"command exceeded {timeout}s deadline: {command[0]}") from error
    except OSError as error:
        raise AcceptanceError(f"command could not start: {command[0]}: {error}") from error


def parser_paths() -> tuple[Path, Path]:
    cargo_home = Path(os.environ.get("CARGO_HOME", str(Path.home() / ".cargo")))
    archives = list((cargo_home / "registry" / "cache").glob("*/html5ever-0.40.1.crate"))
    vcs_files = list((cargo_home / "registry" / "src").glob("*/html5ever-0.40.1/.cargo_vcs_info.json"))
    require(len(archives) == 1, f"expected one cached html5ever {PARSER_VERSION} archive, got {len(archives)}")
    require(len(vcs_files) == 1, f"expected one html5ever {PARSER_VERSION} VCS record, got {len(vcs_files)}")
    return archives[0], vcs_files[0]


def build_inventory() -> dict[str, Any]:
    suite_files = sorted(path for path in SUITE.rglob("*") if path.is_file())
    require(len(suite_files) == EXPECTED_SUITE_INPUT_FILES,
            f"expected {EXPECTED_SUITE_INPUT_FILES} suite inputs, found {len(suite_files)}")
    records = [file_record(path) for path in suite_files]
    records.extend(file_record(path) for path in UPSTREAM_INPUTS)
    records.sort(key=lambda record: record["path"])
    paths = [record["path"] for record in records]
    require(len(paths) == EXPECTED_INVENTORY_FILES,
            f"expected {EXPECTED_INVENTORY_FILES} total inventory inputs, found {len(paths)}")
    require(len(paths) == len(set(paths)), "duplicate source inventory path")

    revision = (RCDOM / "data" / "test" / "wpt-commit.txt").read_text(encoding="ascii").strip()
    require(revision == WPT_REVISION, f"WPT revision file names {revision}, not {WPT_REVISION}")
    head = run_command(["git", "-C", str(HTML5EVER_ROOT), "rev-parse", "HEAD"], 15)
    require(head.returncode == 0 and head.stdout.strip() == HTML5EVER_SOURCE_REVISION,
            f"html5ever source HEAD mismatch: exit={head.returncode} head={head.stdout.strip()!r} stderr={head.stderr[-500:]!r}")

    lock = tomllib.loads((ROOT / "Cargo.lock").read_text(encoding="utf-8"))
    packages = [package for package in lock["package"] if package["name"] == "html5ever"]
    require(len(packages) == 1 and packages[0]["version"] == PARSER_VERSION,
            "Cargo.lock does not pin exactly one html5ever 0.40.1 package")
    package = packages[0]
    archive, vcs_path = parser_paths()
    archive_hash = sha256(archive)
    require(package.get("checksum") == archive_hash, "cached html5ever archive does not match Cargo.lock checksum")
    vcs = json.loads(vcs_path.read_text(encoding="utf-8"))
    require(vcs["git"]["sha1"] == PARSER_VCS_REVISION, "registry VCS revision mismatch")

    dat_files = [path for path in suite_files if path.suffix == ".dat"]
    source_cases = sum(len(parse_dat_file(path)) for path in dat_files)
    outcomes = sum(len(expected_case_records(path)) for path in dat_files)
    return {
        "schema": 1,
        "hash_algorithm": "sha256",
        "suite": {
            "upstream": "web-platform-tests/wpt",
            "revision": revision,
            "root": SUITE_RELATIVE,
            "dat_files": len(dat_files),
            "source_cases": source_cases,
            "outcome_cases": outcomes,
        },
        "html5ever_adapter": {
            "source_revision": head.stdout.strip(),
            "runner": "rcdom/tests/html-tree-builder.rs",
            "ignore_list_inspected_not_applied": True,
        },
        "parser": {
            "crate": "html5ever",
            "version": PARSER_VERSION,
            "vcs_revision": vcs["git"]["sha1"],
            "archive_sha256": archive_hash,
            "vcs_record_sha256": sha256(vcs_path),
            "cargo_lock_sha256": sha256(ROOT / "Cargo.lock"),
            "engine_manifest_sha256": sha256(ROOT / "crates" / "engine" / "Cargo.toml"),
        },
        "files": records,
    }


def validate_inventory(expected: dict[str, Any]) -> None:
    require(expected.get("schema") == 1, "unsupported source inventory schema")
    require(expected.get("hash_algorithm") == "sha256", "source inventory hash algorithm mismatch")
    suite = expected.get("suite", {})
    require(suite.get("upstream") == "web-platform-tests/wpt", "suite upstream mismatch")
    require(suite.get("revision") == WPT_REVISION, "suite revision mismatch")
    require(suite.get("root") == SUITE_RELATIVE, "suite root mismatch")
    require(suite.get("dat_files") == EXPECTED_DAT_FILES,
            f"expected {EXPECTED_DAT_FILES} DAT files, inventory names {suite.get('dat_files')}")
    require(suite.get("source_cases") == EXPECTED_SOURCE_CASES,
            f"expected {EXPECTED_SOURCE_CASES} source cases, inventory names {suite.get('source_cases')}")
    require(suite.get("outcome_cases") == EXPECTED_OUTCOMES,
            f"expected {EXPECTED_OUTCOMES} outcomes, inventory names {suite.get('outcome_cases')}")
    adapter = expected.get("html5ever_adapter", {})
    require(adapter.get("source_revision") == HTML5EVER_SOURCE_REVISION, "adapter source revision mismatch")
    require(adapter.get("ignore_list_inspected_not_applied") is True, "upstream ignore list must not be applied")
    require(expected.get("parser", {}).get("vcs_revision") == PARSER_VCS_REVISION, "parser VCS pin mismatch")
    current = build_inventory()
    require(current == expected, "current source bytes or metadata differ from tree3-suite-hashes.json")


def finish_case(fields: dict[str, str], file_name: str) -> dict[str, str]:
    finished = {key: value[:-1] if value.endswith("\n") else value for key, value in fields.items()}
    require("data" in finished and "document" in finished, f"{file_name}: case lacks #data or #document")
    require(not ("script-on" in finished and "script-off" in finished),
            f"{file_name}: contradictory scripting flags")
    return finished


def parse_dat(source: str, file_name: str) -> list[dict[str, str]]:
    final_line_ended = source.endswith("\n")
    if final_line_ended:
        source = source[:-1]
    lines = source.split("\n")
    cases: list[dict[str, str]] = []
    fields: dict[str, str] = {}
    active: str | None = None
    for index, raw_line in enumerate(lines):
        had_lf = index + 1 < len(lines) or final_line_ended
        line = raw_line[:-1] if had_lf and raw_line.endswith("\r") else raw_line
        if line.startswith("#"):
            heading = line[1:].strip()
            require(heading in SECTION_NAMES, f"{file_name}:{index + 1}: unknown section {heading!r}")
            if heading == "data" and fields:
                if active is not None and fields[active]:
                    fields[active] = fields[active][:-1]
                cases.append(finish_case(fields, file_name))
                fields = {}
                active = None
            elif not fields and heading != "data":
                raise AcceptanceError(f"{file_name}:{index + 1}: case does not start with #data")
            require(heading not in fields, f"{file_name}:{index + 1}: duplicate section {heading}")
            fields[heading] = ""
            active = heading
        elif active is not None:
            fields[active] += line
            if index + 1 < len(lines):
                fields[active] += "\n"
        elif line:
            raise AcceptanceError(f"{file_name}:{index + 1}: content precedes #data")
    if fields:
        cases.append(finish_case(fields, file_name))
    require(cases, f"{file_name}: no cases")
    return cases


def parse_dat_file(path: Path) -> list[dict[str, str]]:
    with path.open("rb") as source_file:
        data = source_file.read(MAX_DAT_BYTES + 1)
    require(len(data) <= MAX_DAT_BYTES, f"{path.name}: exceeds {MAX_DAT_BYTES} bytes")
    try:
        source = data.decode("utf-8")
    except UnicodeDecodeError as error:
        raise AcceptanceError(f"{path.name}: not UTF-8: {error}") from error
    return parse_dat(source, path.name)


def expected_case_records(path: Path) -> list[dict[str, Any]]:
    records = []
    for index, case in enumerate(parse_dat_file(path)):
        modes = (True,) if "script-on" in case else (False,) if "script-off" in case else (False, True)
        for scripting in modes:
            mode = "on" if scripting else "off"
            records.append({
                "kind": "case",
                "id": f"{path.name}#{index}@{mode}",
                "file": path.name,
                "index": index,
                "scripting": scripting,
                "fragment": case.get("document-fragment"),
                "unsupported": path.name.startswith("scripted_"),
                "document": case["document"],
            })
    return records


def expected_census() -> dict[str, dict[str, Any]]:
    files = sorted(SUITE.glob("*.dat"), key=lambda path: path.name)
    require(len(files) == EXPECTED_DAT_FILES, f"actual corpus has {len(files)} DAT files")
    records = [record for path in files for record in expected_case_records(path)]
    require(len(records) == EXPECTED_OUTCOMES, f"actual corpus expands to {len(records)} outcomes")
    census = {record["id"]: record for record in records}
    require(len(census) == len(records), "duplicate expected case identity")
    return census


def validate_report_lines(lines: list[dict[str, Any]], census: dict[str, dict[str, Any]]) -> dict[str, int]:
    require(len(lines) >= 2, "report lacks metadata or summary")
    metadata, summary = lines[0], lines[-1]
    require(set(metadata) == {"kind", "schema", "engine_version", "suite", "suite_revision_label", "files", "timeout_ms", "limits"},
            "metadata fields differ from schema 1")
    require(metadata["kind"] == "metadata" and metadata["schema"] == 1, "metadata kind/schema mismatch")
    require(metadata["suite_revision_label"] == WPT_REVISION, "report suite revision mismatch")
    require(metadata["files"] == EXPECTED_DAT_FILES, "report DAT file count mismatch")
    require(metadata["timeout_ms"] == TIMEOUT_MS, "report subprocess deadline mismatch")
    require(metadata["limits"] == LIMITS, "report limits are not explicit")
    require(comparable_path(Path(metadata["suite"])) == comparable_path(SUITE),
            "report suite path differs from the inventoried corpus")
    workspace = tomllib.loads((ROOT / "Cargo.toml").read_text(encoding="utf-8"))
    require(metadata["engine_version"] == workspace["workspace"]["package"]["version"], "engine version mismatch")

    actual: dict[str, dict[str, Any]] = {}
    required_fields = {"kind", "id", "file", "index", "scripting", "fragment", "status", "detail"}
    for record in lines[1:-1]:
        require(record.get("kind") == "case", f"non-case/file error record present: {record}")
        require(set(record) == required_fields, f"case fields differ from schema 1: {record.get('id')}")
        case_id = record["id"]
        require(isinstance(case_id, str) and case_id not in actual, f"duplicate/invalid case identity: {case_id}")
        require(case_id in census, f"unexpected case identity: {case_id}")
        expected = census[case_id]
        for field in ("file", "index", "scripting", "fragment"):
            require(record[field] == expected[field], f"{case_id}: {field} differs from source")
        require(record["status"] in OUTCOMES, f"{case_id}: invalid status {record['status']}")
        require(isinstance(record["detail"], str), f"{case_id}: detail is not a string")
        require(record["status"] != "unsupported" or expected["unsupported"],
                f"{case_id}: non-script-execution case marked unsupported")
        if expected["unsupported"]:
            require(record["status"] == "unsupported", f"{case_id}: script-execution case was not explicitly unsupported")
            require("JavaScript" in record["detail"], f"{case_id}: unsupported reason is not explicit")
        if record["status"] == "pass":
            require(record["detail"] == "", f"{case_id}: pass carries a hidden diagnostic")
        else:
            require(record["detail"], f"{case_id}: non-pass lacks diagnostic detail")
        actual[case_id] = record

    missing = sorted(set(census) - set(actual))
    require(not missing, f"report is missing {len(missing)} cases; first={missing[:3]}")
    require(set(summary) == {"kind", "source_cases", "outcomes"} and summary["kind"] == "summary",
            "summary schema mismatch")
    require(summary["source_cases"] == EXPECTED_SOURCE_CASES, "summary source-case count mismatch")
    counts = {status: 0 for status in OUTCOMES}
    for record in actual.values():
        counts[record["status"]] += 1
    require(summary["outcomes"] == counts, f"summary outcome mismatch: reported={summary['outcomes']} derived={counts}")
    for status in BLOCKED_OUTCOMES:
        require(counts[status] == 0, f"operational acceptance blocked by {status}={counts[status]}")
    require(counts["unsupported"] == sum(item["unsupported"] for item in census.values()),
            "unsupported outcome census mismatch")
    return counts


def read_report(census: dict[str, dict[str, Any]]) -> tuple[list[dict[str, Any]], dict[str, int]]:
    require(REPORT.is_file(), "adapter did not create tree3-report.jsonl")
    require(REPORT.stat().st_size <= MAX_REPORT_BYTES, "report exceeds bounded audit size")
    lines = []
    for number, line in enumerate(REPORT.read_text(encoding="utf-8").splitlines(), 1):
        require(line, f"report line {number} is empty")
        try:
            value = json.loads(line)
        except json.JSONDecodeError as error:
            raise AcceptanceError(f"report line {number} is invalid JSON: {error}") from error
        require(isinstance(value, dict), f"report line {number} is not an object")
        lines.append(value)
    return lines, validate_report_lines(lines, census)


def negative_controls(lines: list[dict[str, Any]], census: dict[str, dict[str, Any]]) -> None:
    controls: list[tuple[str, list[dict[str, Any]]]] = []
    missing = lines.copy()
    del missing[1]
    controls.append(("missing_case", missing))
    duplicate = lines.copy()
    duplicate[1] = duplicate[2]
    controls.append(("duplicate_case", duplicate))
    bad_metadata = lines.copy()
    bad_metadata[0] = {**bad_metadata[0], "suite_revision_label": "0" * 40}
    controls.append(("source_revision_mismatch", bad_metadata))
    for name, status in (("harness_error_visible", "harness_error"), ("timeout_visible", "timeout"),
                         ("unexpected_unsupported", "unsupported"), ("hidden_outcome", "skip")):
        changed = lines.copy()
        case_index = next(index for index, record in enumerate(changed) if record.get("status") == "pass")
        changed[case_index] = {**changed[case_index], "status": status, "detail": "negative control"}
        controls.append((name, changed))
    bad_summary = lines.copy()
    bad_summary[-1] = {**bad_summary[-1], "source_cases": EXPECTED_SOURCE_CASES - 1}
    controls.append(("summary_mismatch", bad_summary))
    for name, candidate in controls:
        try:
            validate_report_lines(candidate, census)
        except AcceptanceError:
            continue
        raise AcceptanceError(f"negative control was accepted: {name}")
    print(f"TREE3_NEGATIVE_CONTROLS_OK count={len(controls)}")


def write_inventory() -> None:
    inventory = build_inventory()
    validate_inventory(inventory)
    MANIFEST.write_text(json.dumps(inventory, indent=2) + "\n", encoding="utf-8", newline="\n")
    print(f"TREE3_INVENTORY_WRITTEN files={len(inventory['files'])} path={MANIFEST}")


def accept() -> None:
    require(MANIFEST.is_file(), f"missing pinned inventory {MANIFEST}; run --write-inventory intentionally")
    expected = json.loads(MANIFEST.read_text(encoding="utf-8"))
    validate_inventory(expected)
    census = expected_census()
    print(f"TREE3_SOURCE_INVENTORY_OK files={len(expected['files'])} dat_files={EXPECTED_DAT_FILES} "
          f"source_cases={EXPECTED_SOURCE_CASES} outcomes={EXPECTED_OUTCOMES}")

    build = run_command([
        "cargo", "build", "--target-dir", str(TARGET), "-p", "opensequoia-engine",
        "--locked", "--example", "tree_construction",
    ], 240)
    require(build.returncode == 0, f"adapter build failed: {build.stdout[-1000:]}{build.stderr[-2000:]}")
    print("TREE3_ADAPTER_BUILD_OK")

    run = run_command([
        str(ADAPTER), "--suite", str(SUITE), "--suite-revision", WPT_REVISION,
        "--report", str(REPORT), "--scratch-dir", str(TARGET / "case-runs"),
        "--timeout-ms", str(TIMEOUT_MS),
    ], 900)
    require(run.returncode in (0, 1), f"adapter operational failure exit={run.returncode}: {run.stderr[-2000:]}")
    require(run.stderr == "", f"adapter emitted operational stderr: {run.stderr[-2000:]}")
    lines, counts = read_report(census)
    expected_stdout = "TREE_CONSTRUCTION_REPORT cases={} {{{}}}".format(
        EXPECTED_SOURCE_CASES,
        ",".join(f"{json.dumps(status)}:{counts[status]}" for status in sorted(OUTCOMES)),
    )
    require(run.stdout.splitlines() == [expected_stdout],
            f"adapter stdout/report mismatch: {run.stdout!r} != {expected_stdout!r}")
    print(f"TREE3_ADAPTER_EXIT={run.returncode} (genuine fail/unsupported outcomes remain nonzero)")
    print("TREE3_OUTCOMES " + " ".join(f"{status}={counts[status]}" for status in OUTCOMES))
    negative_controls(lines, census)
    validate_inventory(expected)
    print("S04A_ACCEPTANCE_OK")


def main() -> int:
    try:
        if sys.argv[1:] == ["--write-inventory"]:
            write_inventory()
        elif sys.argv[1:]:
            raise AcceptanceError("usage: tree3-acceptance.py [--write-inventory]")
        else:
            accept()
    except (AcceptanceError, OSError, KeyError, TypeError, ValueError, json.JSONDecodeError) as error:
        print(f"S04A_ACCEPTANCE_ERROR: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
