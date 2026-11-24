from concurrent.futures import ThreadPoolExecutor
from datetime import datetime, timezone
from hashlib import sha256
from html.parser import HTMLParser
import json
from pathlib import Path
from urllib.request import urlopen

class Text(HTMLParser):
    def __init__(self):
        super().__init__()
        self.text = []
    def handle_data(self, value):
        self.text.append(value)

root = Path(__file__).resolve().parent
sources = {
    "attributes": "https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-updateprocthreadattribute",
    "job-limits": "https://learn.microsoft.com/en-us/windows/win32/api/jobapi2/nf-jobapi2-setinformationjobobject",
    "creation": "https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-createprocessw",
    "cancel": "https://learn.microsoft.com/en-us/windows/win32/api/ioapiset/nf-ioapiset-cancelsynchronousio",
    "jobs": "https://learn.microsoft.com/en-us/windows/win32/procthread/job-objects",
}

def fetch(item):
    name, url = item
    with urlopen(url, timeout=30) as response:
        data = response.read(2 * 1024 * 1024)
    parser = Text()
    parser.feed(data.decode("utf-8"))
    text = " ".join(parser.text)
    (root / f"process2-doc-{name}.txt").write_text(text, encoding="utf-8")
    snippets = []
    for word in ("PROC_THREAD_ATTRIBUTE_JOB_LIST", "PROC_THREAD_ATTRIBUTE_HANDLE_LIST", "JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE", "JOB_OBJECT_LIMIT_ACTIVE_PROCESS", "ERROR_NOT_FOUND", "does not wait", "last job handle", "Windows 10"):
        offset = text.find(word)
        if offset >= 0:
            snippets.append({"term": word, "excerpt": text[max(0, offset - 50):offset + 1400]})
    return {"name": name, "url": url, "retrieved": datetime.now(timezone.utc).isoformat(), "html_sha256": sha256(data).hexdigest(), "snippets": snippets}

with ThreadPoolExecutor(max_workers=3) as pool:
    records = list(pool.map(fetch, sources.items()))
(root / "process2-official-docs.json").write_text(json.dumps(records, indent=2), encoding="utf-8")
for record in records:
    print(record["url"])
    for snippet in record["snippets"]:
        print(snippet["excerpt"])
