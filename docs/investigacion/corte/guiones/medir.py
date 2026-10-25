"""Measures a cutter's decisions against the parts sheet (page-parts/2).

Usage: python -I medir.py <partes.json> <decisions.json> [<name>]

A step "after s" is the step from page s to page s + 1; a cut there means a
document starts at page s + 1. "doubtful" counts as continue. As the sheet
says, a cut just before or just after a blank page counts the same: every cut
moves forward past the blank pages that follow it. Cuts inside a page (two
documents in regions of one page) are out of reach of a page cutter and are
counted apart. Prints a table and writes nothing; only numbers.
"""
import collections
import json
import sys


def canonical(after, blanks, pages):
    while after + 1 in blanks and after + 1 < pages:
        after += 1
    return after if after < pages else None


def ranges(cuts, pages):
    starts = [1] + [c + 1 for c in sorted(cuts)]
    ends = [s - 1 for s in starts[1:]] + [pages]
    return list(zip(starts, ends))


def measure(sheet_file, dec):
    pages = sheet_file["pages"]
    blanks = {m["page"] for m in sheet_file["page_marks"] if m["form"] == "blank_or_footer"}
    by_range = collections.defaultdict(list)
    for d in sheet_file["documents"]:
        by_range[(d["first_page"], d["last_page"])].append(d["type"])
    truth = {d["first_page"] - 1 for d in sheet_file["documents"] if d["first_page"] > 1}
    inside = sum(len(v) - 1 for v in by_range.values())
    steps = dec["steps"]
    assert [s["after"] for s in steps] == list(range(1, pages)), "every step exactly once"
    raw_cuts = [s["after"] for s in steps if s["decision"] == "new"]
    cuts = {c for c in (canonical(a, blanks, pages) for a in raw_cuts) if c is not None}
    got = ranges(cuts, pages)
    covered = [p for a, b in got for p in range(a, b + 1)]
    integrity = covered == list(range(1, pages + 1))
    types = {}
    for start, t in dec.get("types", {}).items():
        c = canonical(int(start) - 1, blanks, pages) if int(start) > 1 else 0
        if c is not None:
            types[c + 1] = t
    well = [r for r in got if len(by_range.get(r, [])) == 1]
    typed = sum(1 for r in well if types.get(r[0]) == by_range[r][0])
    rules = collections.defaultdict(lambda: [0, 0])
    for s in steps:
        if s["decision"] == "doubtful":
            continue
        if s["decision"] == "new":
            ok = canonical(s["after"], blanks, pages) in truth
        else:
            ok = s["after"] not in truth
        rules[f'{s["rule"]}:{s["decision"]}'][0 if ok else 1] += 1
    return {
        "hits": len(cuts & truth), "extra": len(cuts - truth), "lost": len(truth - cuts), "inside": inside,
        "true_cuts": len(truth), "docs": sum(len(v) for v in by_range.values()), "well_cut": len(well),
        "typed": typed, "integrity": integrity, "doubtful": sum(1 for s in steps if s["decision"] == "doubtful"),
        "steps": len(steps), "rules": dict(rules),
    }


def pct(a, b):
    return f"{100 * a / b:.0f} %" if b else "-"


def main(sheet_path, dec_path, name=None):
    sheet = {f["name"]: f for f in json.load(open(sheet_path, encoding="utf-8"))["files"]}
    decs = json.load(open(dec_path, encoding="utf-8"))
    meta = decs.get("meta", {}) if isinstance(decs, dict) else {}
    files = decs["files"] if isinstance(decs, dict) else decs
    total = collections.Counter()
    rules = collections.defaultdict(lambda: [0, 0])
    print(f"### {name or dec_path}\n")
    print("| Archivo | Pasos | Cortes reales | Aciertos | De más | Perdidos | Precisión | Alcance | Dentro de una página | Bien cortados | Tipo bien | Dudosos | Íntegro |")
    print("|---|---|---|---|---|---|---|---|---|---|---|---|---|")
    for d in files:
        m = measure(sheet[d["file"]], d)
        for k, v in m.items():
            if isinstance(v, (int, bool)) and k != "integrity":
                total[k] += v
        total["broken"] += 0 if m["integrity"] else 1
        for k, (ok, bad) in m["rules"].items():
            rules[k][0] += ok
            rules[k][1] += bad
        print(f'| {d["file"][:2]} | {m["steps"]} | {m["true_cuts"]} | {m["hits"]} | {m["extra"]} | {m["lost"]} | '
              f'{pct(m["hits"], m["hits"] + m["extra"])} | {pct(m["hits"], m["true_cuts"])} | {m["inside"]} | '
              f'{m["well_cut"]} de {m["docs"]} | {m["typed"]} | {m["doubtful"]} | {"sí" if m["integrity"] else "no"} |')
    t = total
    print(f'| Total | {t["steps"]} | {t["true_cuts"]} | {t["hits"]} | {t["extra"]} | {t["lost"]} | '
          f'{pct(t["hits"], t["hits"] + t["extra"])} | {pct(t["hits"], t["true_cuts"])} | {t["inside"]} | '
          f'{t["well_cut"]} de {t["docs"]} | {t["typed"]} | {t["doubtful"]} | {"sí" if not t["broken"] else "no"} |')
    if meta:
        print(f'\nConsultas a Codex: {meta.get("codex_calls", 0)}; segundos de Codex: {meta.get("codex_seconds", 0):.0f}; '
              f'respuestas inválidas: {meta.get("codex_failed", 0)}.')
    print("\n| Regla:decisión | Bien | Mal |\n|---|---|---|")
    for k in sorted(rules):
        print(f"| {k} | {rules[k][0]} | {rules[k][1]} |")
    print()


if __name__ == "__main__":
    main(*sys.argv[1:])
