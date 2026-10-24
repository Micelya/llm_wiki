"""Score label->value pairs of a form against a private answer file, printing only neutral pair names.
A pair is right when the value appears between its label and the next label in the text, with tables flattened.
Usage: python -I pares01.py <answers.json> <text_or_md>... ; prints JSON {file: {ok, total, wrong:[names]}}"""
import json, re, sys, unicodedata

def norm(s):
    s = unicodedata.normalize("NFKD", s)
    s = "".join(c for c in s if not unicodedata.combining(c))
    return re.sub(r"[^A-Z0-9@+/]", "", s.upper())

gold = json.load(open(sys.argv[1], encoding="utf-8"))["pairs"]
labels = []
for lab, _, _ in gold:
    base, _, occ = lab.partition("#")
    labels.append((norm(base), int(occ or 1)))
all_labels = {l for l, _ in labels} | {norm(x) for x in ["Nombre completo", "Sírvase indicar", "Por cuenta propia", "Nro. Doc", "Vinculación", "Lugar de nacimiento", "Estado Civil"]}
res = {}
for path in sys.argv[2:]:
    t = open(path, encoding="utf-8").read().replace("|", " ")
    t = re.sub(r"<!--.*?-->", " ", t)
    # one string with markers per label position
    n = norm(t)
    ok, wrong = 0, []
    for (lab, occ), (_, val, name) in zip(labels, gold):
        starts = [m.start() for m in re.finditer(re.escape(lab), n)]
        # avoid "Cargo" matching inside other labels: keep plain occurrences
        if len(starts) < occ:
            wrong.append(name); continue
        s = starts[occ - 1] + len(lab)
        nxt = min([p for L in all_labels for p in [n.find(L, s)] if p >= 0 and L != lab] + [len(n)])
        nxt2 = n.find(lab, s)
        if nxt2 >= 0: nxt = min(nxt, nxt2)
        if norm(val) in n[s:nxt]:
            ok += 1
        else:
            wrong.append(name)
    res[path.split("/")[-1] if "/" not in path[-60:] else path[-60:]] = {"ok": ok, "total": len(gold), "wrong": wrong}
print(json.dumps(res, ensure_ascii=False, indent=1))
