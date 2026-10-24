"""Find an ICAO 9303 TD1 machine-readable zone (ID card, 3 lines of 30) in any OCR text, even when the tool
glued the lines together or added spaces, and check its digits. Prints only verdicts, never the data.
Usage: python -I mrz2.py <text>...  -> one JSON per file"""
import json, re, sys
W = [7, 3, 1]
def val(c): return int(c) if c.isdigit() else 0 if c == "<" else ord(c) - 55
def check(s): return sum(val(c) * W[i % 3] for i, c in enumerate(s)) % 10
def ok(s, d): return d.isdigit() and check(s) == int(d)
for path in sys.argv[1:]:
    raw = open(path, encoding="utf-8").read().upper()
    t = re.sub(r"\s+", "", raw).replace("«", "<").replace("‹", "<")
    r = {"archivo": path, "renglon1": False, "renglon2": False, "renglon3": False}
    m1 = re.search(r"I[DC<][A-Z<]{3}[A-Z0-9<]{9}[0-9<][A-Z0-9<]{15}", t)
    m2 = re.search(r"\d{6}\d[MF<]\d{6}\d[A-Z<]{3}[A-Z0-9<]{11}\d", t)
    m3 = re.search(r"[A-Z]+<<[A-Z<]{5,}", t[m2.end():] if m2 else t)
    if m1:
        l1 = m1.group(0)[:30]; r["renglon1"] = True
        r["digito_documento"] = ok(l1[5:14], l1[14])
    if m2:
        l2 = m2.group(0)[:30]; r["renglon2"] = True
        r["digito_nacimiento"] = ok(l2[0:6], l2[6]); r["digito_vencimiento"] = ok(l2[8:14], l2[14])
        if m1:
            r["digito_compuesto"] = ok(l1[5:30] + l2[0:7] + l2[8:15] + l2[18:29], l2[29])
    if m3: r["renglon3"] = True
    r["entera_y_valida"] = all(r.get(k) for k in ("renglon1", "renglon2", "renglon3", "digito_documento", "digito_nacimiento", "digito_vencimiento", "digito_compuesto"))
    print(json.dumps(r))
