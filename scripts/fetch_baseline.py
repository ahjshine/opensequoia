"""Download the pinned public Brave distribution and verify publisher digest."""
import hashlib
import json
from pathlib import Path
import urllib.request

URL = "https://github.com/brave/brave-browser/releases/download/v1.100.58/BraveBrowserStandaloneSetup.exe"
SHA256 = "2802553c89831effe624bd1b0dd8e39c463e03a1224d557441350a5586e559ce"
BYTES = 173087584
workspace = Path(__file__).resolve().parents[1]
destination = workspace / "evidence/artifacts/raw/BraveBrowserStandaloneSetup-1.100.58.exe"
destination.parent.mkdir(parents=True, exist_ok=True)
digest = hashlib.sha256()
if not destination.exists():
    with urllib.request.urlopen(URL, timeout=60) as response, destination.open("wb") as output:
        while chunk := response.read(1024 * 1024):
            output.write(chunk)
with destination.open("rb") as source:
    while chunk := source.read(1024 * 1024):
        digest.update(chunk)
if destination.stat().st_size != BYTES or digest.hexdigest() != SHA256:
    raise RuntimeError("artifact size or SHA256 differs from pinned publisher metadata")
report = {"release": "v1.100.58", "url": URL, "absolute_path": str(destination),
          "bytes": BYTES, "sha256": digest.hexdigest(), "publisher_digest_matched": True,
          "limitation": "Digest verifies correspondence to release metadata; does not establish runtime behavior, PE architecture or security."}
(workspace / "evidence/artifacts/brave-download.json").write_text(json.dumps(report, indent=2), encoding="utf-8")
print(json.dumps(report))
