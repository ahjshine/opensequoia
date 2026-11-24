"""Package the current local renderer development artifact with license texts."""
import hashlib
import argparse
import json
import re
from pathlib import Path
import shutil
import subprocess
import zipfile


def main():
    workspace = Path(__file__).resolve().parents[1]
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--executable", type=Path, default=workspace / "target/release/opensequoia.exe")
    args = parser.parse_args()
    metadata = json.loads(subprocess.check_output(
        ["cargo", "metadata", "--locked", "--format-version", "1"], cwd=workspace))
    executable = args.executable.resolve()
    version = next(p["version"] for p in metadata["packages"] if p["name"] == "opensequoia")
    if re.fullmatch(r"[0-9]+\.[0-9]+\.[0-9]+", version) is None:
        raise RuntimeError("development package version must be three numeric components")
    label = f"opensequoia-{version}-win-x64"
    if not executable.is_file():
        raise RuntimeError("run cargo build --release --locked first")
    notices = ["OpenSequoia development payload, MIT. No Brave affiliation or trademark license implied.",
               "Included registry crate licenses (conservative all-target/build inventory; not a claim every crate is linked):"]
    inventory = []
    for package in sorted(metadata["packages"], key=lambda p: p["name"]):
        if package["source"] is None:
            continue
        source = Path(package["manifest_path"]).parent
        files = sorted(set(source.glob("LICENSE*")) | set(source.glob("COPYING*")))
        texts = [(file.name, file.read_text(encoding="utf-8")) for file in files if file.is_file()]
        if not texts:
            raise RuntimeError(f"no distributable license text found for {package['name']}")
        inventory.append({"name": package["name"], "version": package["version"],
                          "license": package["license"], "source": package["source"],
                          "license_files": [name for name, _ in texts]})
        notices.extend([f"\n===== {package['name']} {package['version']} ({package['license']}) ====="])
        for name, value in texts:
            notices.extend([f"--- {name} ---", value])
    output = workspace / "evidence/artifacts/raw" / label
    output.mkdir(parents=True, exist_ok=True)
    shutil.copy2(executable, output / "opensequoia.exe")
    shutil.copy2(workspace / "LICENSE", output / "LICENSE")
    shutil.copy2(workspace / "README.md", output / "README.md")
    (output / "THIRD_PARTY_NOTICES.txt").write_text("\n".join(notices), encoding="utf-8")
    payload_files = [output / name for name in ["opensequoia.exe", "LICENSE", "README.md", "THIRD_PARTY_NOTICES.txt"]]
    archive = workspace / "evidence/artifacts/raw" / f"{label}.zip"
    with zipfile.ZipFile(archive, "w", compression=zipfile.ZIP_DEFLATED, compresslevel=9) as bundle:
        for file in payload_files:
            info = zipfile.ZipInfo(file.name, date_time=(1980, 1, 1, 0, 0, 0))
            info.create_system = 3
            info.external_attr = 0o644 << 16
            bundle.writestr(info, file.read_bytes(), compress_type=zipfile.ZIP_DEFLATED, compresslevel=9)
    report = {"kind": "unsigned development renderer payload; not browser release/installer",
              "version": version, "target": "x86_64-pc-windows-msvc",
              "executable_bytes": executable.stat().st_size,
              "build_executable": str(executable),
              "executable_sha256": hashlib.sha256(executable.read_bytes()).hexdigest(),
              "payload_bytes": sum(file.stat().st_size for file in payload_files),
              "zip_bytes": archive.stat().st_size,
              "zip_sha256": hashlib.sha256(archive.read_bytes()).hexdigest(),
              "zip_metadata": "fixed order, timestamp 1980-01-01, Unix regular file mode 0644; reproducible for identical inputs/tool runtime",
              "archive": str(archive), "files": [file.name for file in payload_files],
              "limitations": ["Does not include production browser retained scope", "Not comparable to full Brave payload",
                               "No signing/update/install support", "No fresh-install footprint or allocated-size claim"],
              "registry_inventory": inventory}
    (workspace / "evidence/artifacts/opensequoia-build.json").write_text(json.dumps(report, indent=2), encoding="utf-8")
    (workspace / f"evidence/artifacts/opensequoia-{version}-build.json").write_text(json.dumps(report, indent=2), encoding="utf-8")
    (workspace / "evidence/artifacts/THIRD_PARTY_NOTICES.txt").write_text("\n".join(notices), encoding="utf-8")
    print(json.dumps({key: value for key, value in report.items() if key != "registry_inventory"}, indent=2))


if __name__ == "__main__":
    main()
