import json, pathlib
def load(name):
    d=json.loads(pathlib.Path(f"evidence/artifacts/sandbox4-craft-{name}.json").read_text(encoding="utf-8"))
    return json.loads(d["stdout"])
for pair,label in [(["overview","old-overview"],"overview"),(["layout","old-layout"],"layout"),(["resources","old-resources"],"resources"),(["imports","old-imports"],"imports")]:
    new,old=load(pair[0]),load(pair[1])
    print("="*10,label,"="*10)
    print("new",new["subject"]["sha256"][:12],new["id"])
    print("old",old["subject"]["sha256"][:12],old["id"])
    if label=="overview":
        nr,orr=new["result"],old["result"]
        print("bytes",new.get("subject"),old.get("subject"))
        print("counts new",nr["counts"])
        print("counts old",orr["counts"])
        print("libs new",sorted(nr["libraries"]))
        print("libs old",sorted(orr["libraries"]))
        print("new only",sorted(set(nr["libraries"])-set(orr["libraries"])))
        print("old only",sorted(set(orr["libraries"])-set(nr["libraries"])))
        print("mit new",nr["mitigations"])
        print("mit old",orr["mitigations"])
        print("info new",nr["info"])
        print("info old",orr["info"])
    if label=="resources":
        print("new",new["result"]["version"],new["result"]["manifest"][:120])
        print("old",old["result"]["version"],old["result"]["manifest"][:120])
    if label=="imports":
        print("coverage",new["coverage"],old["coverage"])