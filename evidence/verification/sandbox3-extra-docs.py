import hashlib
import json
import re
import urllib.request
from pathlib import Path

# Use the already-reviewed text extraction rules without importing the downloader.
from html.parser import HTMLParser


class Text(HTMLParser):
    def __init__(self):
        super().__init__()
        self.parts = []
        self.skip = 0

    def handle_starttag(self, tag, attrs):
        if tag in ("script", "style"):
            self.skip += 1

    def handle_endtag(self, tag):
        if tag in ("script", "style"):
            self.skip = max(0, self.skip - 1)
        if tag in ("p", "li", "tr", "div", "h1", "h2", "h3", "pre"):
            self.parts.append("\n")

    def handle_data(self, text):
        if not self.skip:
            self.parts.append(text)


isolation = urllib.request.urlopen(
    "https://learn.microsoft.com/en-us/windows/win32/secauthz/appcontainer-isolation",
    timeout=25,
).read().decode()
links = re.findall(r'href="([^"]*appcontainer[^"]*)"', isolation)
Path("evidence/verification/sandbox3-isolation-links.json").write_text(
    json.dumps(links, indent=2) + "\n", encoding="utf-8"
)
print(json.dumps(links))

urls = [
    ("legacy", "https://learn.microsoft.com/en-us/windows/win32/secauthz/appcontainer-for-legacy-applications-"),
    ("network", "https://learn.microsoft.com/en-us/windows/win32/winsock/network-isolation"),
]
records = []
for name, url in urls:
    try:
        raw = urllib.request.urlopen(url, timeout=25).read()
        parser = Text()
        parser.feed(raw.decode("utf-8"))
        text = "\n".join(
            " ".join(line.split())
            for line in "".join(parser.parts).splitlines()
            if line.strip()
        )
        out = Path("evidence/verification/sandbox3-doc-" + name + ".txt")
        out.write_text(text + "\n", encoding="utf-8")
        records.append({"name": name, "url": url, "path": str(out), "result": "ok", "sha256_html": hashlib.sha256(raw).hexdigest(), "sha256_text": hashlib.sha256(out.read_bytes()).hexdigest()})
    except Exception as error:
        records.append({"name": name, "url": url, "result": repr(error)})
Path("evidence/verification/sandbox3-extra-docs.json").write_text(
    json.dumps(records, indent=2) + "\n", encoding="utf-8"
)
print(json.dumps(records, indent=2))
