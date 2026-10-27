"""Busca datos reales en las líneas que una rama agrega sobre main, antes de subirla. El repositorio es público.

Uso: python revisar-datos-reales.py <datos> [rama]
  <datos>: la carpeta datos/ que dejó bajar-de-drive.py, con pruebas/revision-de-datos, pruebas/hojas-de-respuestas y pruebas/base.
  rama:    lo que se revisa; por defecto HEAD (lo commiteado, comparado con origin/main).
Busca:
  - los nombres de personas, organizaciones y casos de la base, las palabras sensibles de los PDF de prueba
    y los nombres y valores de las hojas de respuestas;
  - números con forma de DNI o CUIT, correos, teléfonos y enlaces o ids de Drive.
Una palabra que ya está en los documentos de main cuenta como común y no se informa.
Cada coincidencia se mira a mano: si es una palabra común, sigue; si es un dato real, se saca o se cambia por un ejemplo inventado.
"""
import json
import os
import re
import sqlite3
import subprocess
import sys
import tempfile
import unicodedata
import zipfile

sys.stdout.reconfigure(encoding="utf-8")


def norm(s):
    return "".join(c for c in unicodedata.normalize("NFKD", s.lower()) if not unicodedata.combining(c))


def palabras(s):
    return re.findall(r"[a-zñ]{4,}", norm(s))


def git(*args):
    return subprocess.run(["git", *args], capture_output=True, text=True, encoding="utf-8", errors="replace").stdout


def leer(path):
    return json.load(open(path, encoding="utf-8")) if os.path.exists(path) else None


def main():
    datos = sys.argv[1]
    rama = sys.argv[2] if len(sys.argv) > 2 else "HEAD"
    hojas = os.path.join(datos, "pruebas", "hojas-de-respuestas")
    buscadas, numeros, fuentes = set(), set(), []

    lista = leer(os.path.join(datos, "pruebas", "revision-de-datos", "palabras-sensibles.json"))
    if lista:
        buscadas.update(w for w in map(norm, lista) if len(w) >= 4)
        fuentes.append("palabras sensibles")
    for nombre in ("entidades.json", "entidades-sucesion.json"):
        hoja = leer(os.path.join(hojas, nombre))
        for e in (hoja or {}).get("entities", []):
            if e.get("type") != "persona":
                continue
            for n in [e.get("name") or ""] + list(e.get("aliases") or []):
                buscadas.update(palabras(n))
            for i in e.get("identifiers") or []:
                v = re.sub(r"\D", "", i.get("value", "") if isinstance(i, dict) else str(i))
                if len(v) >= 6:
                    numeros.add(v)
        if hoja:
            fuentes.append(nombre)
    for valores in (leer(os.path.join(hojas, "manuscrito.json")) or {}).values():
        for v in valores:
            buscadas.update(palabras(v))
            d = re.sub(r"\D", "", v)
            if len(d) >= 4:
                numeros.add(d)
    zpath = os.path.join(datos, "pruebas", "base", "micelya-base.zip")
    if os.path.exists(zpath):
        with tempfile.TemporaryDirectory() as tmp:
            zipfile.ZipFile(zpath).extract("micelya-base.sqlite3", tmp)
            db = sqlite3.connect(os.path.join(tmp, "micelya-base.sqlite3"))
            for (n,) in db.execute("select name from knowledge_entity"):
                buscadas.update(palabras(n))
            db.close()
        fuentes.append("nombres de la base")
    if not buscadas:
        sys.exit("no encontré ninguna lista: bajá pruebas/revision-de-datos, pruebas/hojas-de-respuestas y pruebas/base")

    comunes = set()
    for f in git("ls-tree", "-r", "--name-only", "origin/main").splitlines():
        if f.endswith((".md", ".json")) and (f.startswith("docs/") or "/" not in f):
            comunes.update(palabras(git("show", f"origin/main:{f}")))
    buscadas -= comunes

    patrones = {
        "dni o cuit": re.compile(r"\b\d{1,2}\.?\d{3}\.?\d{3}\b|\b\d{2}-\d{8}-\d\b"),
        "correo": re.compile(r"[\w.+-]+@[\w-]+\.[\w.]+"),
        "telefono": re.compile(r"\b(?:11|15)[ -]?\d{4}[ -]?\d{4}\b"),
        "enlace de drive": re.compile(r"drive\.google\.com/\S*(?:folders/|id=)[\w-]{10,}"),
        "id de drive": re.compile(r"\b(?=[\w-]*[A-Z])(?=[\w-]*[a-z])(?=[\w-]*\d)[\w-]{28,}\b"),
    }
    hallados = agregadas = 0
    archivo, n = None, 0
    for linea in git("diff", "-U0", f"origin/main...{rama}").splitlines():
        if linea.startswith("+++ "):
            archivo = linea[6:]
            continue
        if linea.startswith("@@"):
            n = int(re.search(r"\+(\d+)", linea).group(1)) - 1
            continue
        if not linea.startswith("+"):
            continue
        n += 1
        agregadas += 1
        linea = linea[1:]
        for tipo, p in patrones.items():
            for m in p.findall(linea):
                if "noreply@anthropic" not in m:
                    hallados += 1
                    print(f"{archivo}:{n} {tipo}: {m}")
        for w in palabras(linea):
            if w in buscadas:
                hallados += 1
                print(f"{archivo}:{n} palabra: {w}")
        digitos = re.findall(r"\d{4,}", linea)
        for t in numeros:
            if any(t == d or (len(t) >= 6 and t in d) for d in digitos):
                hallados += 1
                print(f"{archivo}:{n} numero: {t}")
    print(f"revisado {rama} contra origin/main: {agregadas} líneas agregadas; {len(buscadas)} palabras buscadas "
          f"({', '.join(fuentes)}), {len(numeros)} números; coincidencias: {hallados}")


if __name__ == "__main__":
    main()
