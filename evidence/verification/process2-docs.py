from pathlib import Path
from html.parser import HTMLParser

class Text(HTMLParser):
    def __init__(self):
        super().__init__()
        self.text = []
    def handle_data(self, value):
        self.text.append(value)

base = Path(r"C:\Users\deepa\.rustup\toolchains\stable-x86_64-pc-windows-msvc\share\doc\rust\html\std\os\windows\process")
for name in ("trait.CommandExt.html", "struct.ProcThreadAttributeList.html", "struct.ProcThreadAttributeListBuilder.html"):
    parser = Text()
    parser.feed((base / name).read_text(encoding="utf-8"))
    text = " ".join(parser.text)
    print("\n" + name)
    for term in ("spawn_with_attributes(", "raw_attribute(", "pub fn attribute", "pub unsafe fn", "Safety", "1.96", "1.90", "1.94"):
        index = text.find(term)
        if index >= 0:
            print(text[max(0, index - 100):index + 2500])
