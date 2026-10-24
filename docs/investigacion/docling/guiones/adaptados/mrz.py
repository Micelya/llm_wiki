"""Find an ICAO 9303 TD1 machine-readable zone in OCR text and check its digits, without printing personal data.
Usage: python -I mrz.py <ocr.txt>..."""
import re, sys
sys.stdout.reconfigure(encoding="utf-8")
W = [7, 3, 1]
def val(c):
    if c.isdigit(): return int(c)
    if c == "<": return 0
    return ord(c) - 55
def check(s): return sum(val(c) * W[i % 3] for i, c in enumerate(s)) % 10
def clean(l): return re.sub(r"\s", "", l.upper()).replace("«", "<").replace("‹", "<")
for path in sys.argv[1:]:
    lines = [clean(l) for l in open(path, encoding="utf-8-sig").read().splitlines()[1:]]
    cand = [l for l in lines if len(l) >= 25 and l.count("<") >= 2 and re.fullmatch(r"[A-Z0-9<]+", l)]
    print(path.split("/")[-1], "renglones con forma de zona de lectura:", len(cand), [len(c) for c in cand])
    l1 = next((c for c in cand if c.startswith("ID")), None)
    l2 = next((c for c in cand if re.match(r"\d{6}\d[MF<]\d{6}\d", c)), None)
    if l1 and len(l1) >= 15:
        doc, cd = l1[5:14], l1[14]
        print("  renglón 1: tipo", l1[:2], "país", l1[2:5], "| dígito del número de documento:", "bien" if cd.isdigit() and check(doc) == int(cd) else "mal")
    if l2 and len(l2) >= 15:
        print("  renglón 2: dígito de nacimiento", "bien" if check(l2[0:6]) == int(l2[6]) else "mal", "| dígito de vencimiento", "bien" if l2[14].isdigit() and check(l2[8:14]) == int(l2[14]) else "mal")
    if l1 and l2 and len(l1) == 30 and len(l2) == 30:
        comp = l1[5:30] + l2[0:7] + l2[8:15] + l2[18:29]
        print("  dígito compuesto:", "bien" if l2[29].isdigit() and check(comp) == int(l2[29]) else "mal")
