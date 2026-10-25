"""After the cut: relations, the check against what a filing announces, and
the "to look at" mark (D-527). No model; it reuses the cutter's decisions.

Usage: python -I controles.py <senales.json> <decisions.json>
           [--desc <desc.json>] [--sheet <partes.json>] [--out <out.json>]

- Relations: a document whose type follows another (a certification, a
  signature sheet, a signing certificate, a receipt; relaciones.json) proposes
  a relation with the document before it.
- Announced: in each document, after an announcing word ("adjunto",
  "acompaño", "anexo"...), the longest list a), b), c)... / 1., 2., ... /
  i), ii)... or bullets, or the distinct "Anexo X" labels. The documents next
  to it (after, else before) up to a court piece or another announcing
  document form its group; if the group has as many documents as announced,
  the cuts inside it are confirmed, else they are put in doubt.
- To look at: a cut decided by Codex, with no relation between its two sides,
  that the announced check does not confirm.
Types come from the decisions ("types") or, with --desc, from Codex's
descriptions. With --sheet it also measures (only numbers). The output JSON
has pages, types and marks, no text.
"""
import collections
import json
import os
import re
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
from cortar import fold  # noqa: E402
from medir import canonical, ranges  # noqa: E402

VOC = json.load(open(os.path.join(HERE, "relaciones.json"), encoding="utf-8"))
ROMAN = ["i", "ii", "iii", "iv", "v", "vi", "vii", "viii", "ix", "x", "xi", "xii", "xiii", "xiv", "xv"]
MARK = re.compile(r"(?:^|\n)[ \t]*\(?([a-z]|[ivx]{1,4}|\d{1,2})[ \t]*[\)\.\-][ \t]|(?<=[\s:;,])\(([a-z]|\d{1,2})\)\s"
                  r"|(?<=[\s:;,])([a-z]|\d{1,2})\)\s|(?:^|\n)[ \t]*([-•·*])[ \t]")
ANEXO = re.compile(r"\banexo\s+(?:n[oº°.]*\s*)?([a-z]|[ivx]{1,4}|\d{1,2})\b")


def follows(kind, last, m):
    if kind == "letter" and len(m) == 1 and m.isalpha() and ord(m) == ord(last) + 1:
        return kind, m
    if kind == "number" and m.isdigit() and int(m) == last + 1:
        return kind, int(m)
    if kind == "roman" and m in ROMAN and ROMAN.index(m) == last + 1:
        return kind, ROMAN.index(m)
    if kind == "bullet" and m in "-•·*":
        return kind, m
    return None


def start(m):
    return {"a": ("letter", "a"), "1": ("number", 1), "i": ("roman", 0)}.get(m) or \
        (("bullet", m) if m in "-•·*" else None)


def announced(text):
    t = fold(text)
    best = len(set(ANEXO.findall(t)))
    for trig in re.finditer("|".join(map(re.escape, VOC["announce"])), t):
        window = t[trig.start():trig.start() + 3000]
        run, state = 0, None
        for mm in MARK.finditer(window):
            m = next(g for g in mm.groups() if g)
            nxt = follows(*state, m) if state else None
            if nxt:
                state, run = nxt, run + 1
            elif start(m):
                state, run = start(m), 1
            best = max(best, run)
    return best


def file_docs(f, pages_text, blanks, types_of):
    cuts = {c for c in (canonical(s["after"], blanks, f["pages"]) for s in f["steps"] if s["decision"] == "new") if c}
    docs = []
    for a, b in ranges(cuts, f["pages"]):
        docs.append({"from": a, "to": b, "type": types_of(a, b),
                     "announced": announced("\n".join(pages_text[p - 1] for p in range(a, b + 1)))})
    for j, d in enumerate(docs):
        rel = VOC["follows_previous"].get(d["type"]) if j else None
        d["relation"] = rel
    by_cut = {}
    for s in f["steps"]:
        c = canonical(s["after"], blanks, f["pages"]) if s["decision"] == "new" else None
        if c:
            by_cut.setdefault(c + 1, s["rule"])
    for d in docs:
        d["why"] = by_cut.get(d["from"], "start")
    stop = lambda d: d["type"] in VOC["court_pieces"] or d["announced"]  # noqa: E731
    confirmed, doubted, spans = set(), set(), []
    for k, e in enumerate(docs):
        n = e["announced"]
        if not n:
            continue
        after = []
        for d in docs[k + 1:]:
            if stop(d):
                break
            after.append(d)
        before = []
        for d in reversed(docs[:k]):
            if stop(d):
                break
            before.append(d)
        side = after if len(after) == n else before if len(before) == n else None
        e["matches"] = side is not None
        if side is None:
            side = min((s for s in (after, before) if s), key=lambda s: abs(len(s) - n), default=[])
        span = sorted([e] + side, key=lambda d: d["from"])
        inner = {d["from"] - 1 for d in span[1:]}
        (confirmed if e["matches"] else doubted).update(inner)
        if e["matches"]:
            spans.append((span[0]["from"], span[-1]["to"]))
    for d in docs:
        c = d["from"] - 1
        d["confirmed"], d["doubted"] = c in confirmed, c in doubted and c not in confirmed
        codex = d["why"] == "codex"
        d["look"] = codex and not d["relation"] and not d["confirmed"]
    cont = [s["after"] for s in f["steps"] if s["rule"] == "codex" and s["decision"] != "new"]
    cont_look = [a for a in cont if not any(x <= a < y for x, y in spans)]
    return docs, cont_look


def main(argv):
    sen_path, dec_path, *rest = argv
    opts = dict(zip(rest[::2], rest[1::2]))
    sen = {f["file"]: [p.get("text", "") for p in f["pages"]] for f in json.load(open(sen_path, encoding="utf-8"))}
    decs = json.load(open(dec_path, encoding="utf-8"))
    files = decs["files"] if isinstance(decs, dict) else decs
    desc = {}
    if "--desc" in opts:
        for x in json.load(open(opts["--desc"], encoding="utf-8"))["docs"]:
            desc[(x["file"], x["from"], x["to"])] = x["answer"].get("type")
    sheet = {}
    if "--sheet" in opts:
        sheet = {f["name"]: f for f in json.load(open(opts["--sheet"], encoding="utf-8"))["files"]}
    tot, out = collections.Counter(), []
    for f in files:
        sf = sheet.get(f["file"])
        blanks = {m["page"] for m in sf["page_marks"] if m["form"] == "blank_or_footer"} if sf else set()
        if desc:
            types_of = lambda a, b, n=f["file"]: desc.get((n, a, b))  # noqa: E731
        else:
            tmap = {int(k): v for k, v in f.get("types", {}).items()}
            types_of = lambda a, b, t=tmap: t.get(a)  # noqa: E731
        docs, cont_look = file_docs(f, sen[f["file"]], blanks, types_of)
        out.append({"file": f["file"], "docs": docs, "codex_continue_to_look": cont_look})
        tot["docs"] += len(docs)
        tot["relations"] += sum(1 for d in docs if d["relation"])
        tot["announcing"] += sum(1 for d in docs if d["announced"])
        tot["matches"] += sum(1 for d in docs if d.get("matches"))
        tot["confirmed"] += sum(1 for d in docs if d["confirmed"])
        tot["doubted"] += sum(1 for d in docs if d["doubted"])
        tot["codex_cuts"] += sum(1 for d in docs if d["why"] == "codex")
        tot["look"] += sum(1 for d in docs if d["look"])
        tot["codex_continue_look"] += len(cont_look)
        if not sf:
            continue
        truth = {d["first_page"] - 1 for d in sf["documents"] if d["first_page"] > 1}
        ids = collections.defaultdict(list)
        for d in sf["documents"]:
            ids[(d["first_page"], d["last_page"])].append(d)
        pairs = {frozenset((d["id"], r["to"])) for d in sf["documents"] for r in d.get("related", [])}
        tot["sheet_relations"] += len(pairs)
        found = set()
        for j, d in enumerate(docs):
            if not d["relation"]:
                continue
            x, y = ids.get((d["from"], d["to"]), []), ids.get((docs[j - 1]["from"], docs[j - 1]["to"]), [])
            hit = next((frozenset((a["id"], b["id"])) for a in x for b in y
                        if frozenset((a["id"], b["id"])) in pairs), None)
            if hit:
                found.add(hit)
            else:
                tot["relations_extra"] += 1
        tot["relations_found"] += len(found)
        for d in docs:
            right = (d["from"] - 1) in truth
            tot["confirmed_right"] += d["confirmed"] and right
            tot["doubted_wrong"] += d["doubted"] and not right
            tot["look_wrong"] += d["look"] and not right
            tot["codex_cut_wrong"] += d["why"] == "codex" and not right
        tot["codex_continue_look_wrong"] += sum(1 for a in cont_look if a in truth)
        tot["codex_continue_wrong"] += sum(1 for s in f["steps"] if s["rule"] == "codex"
                                           and s["decision"] != "new" and s["after"] in truth)
        parents = collections.Counter(d["attached_to"] for d in sf["documents"] if d.get("attached_to"))
        for d in sf["documents"]:
            if d["id"] in parents:
                n = announced("\n".join(sen[f["file"]][p - 1] for p in range(d["first_page"], d["last_page"] + 1)))
                tot["sheet_parents"] += 1
                tot["sheet_parents_announcing"] += n > 0
                tot["sheet_parents_exact"] += n == parents[d["id"]]
    for k in sorted(tot):
        print(f"{k}: {tot[k]}")
    if "--out" in opts:
        json.dump(out, open(opts["--out"], "w", encoding="utf-8"), ensure_ascii=False, indent=1)


if __name__ == "__main__":
    main(sys.argv[1:])
