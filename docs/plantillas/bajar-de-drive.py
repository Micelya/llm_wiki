"""Baja a disco una carpeta compartida de Google Drive, o solo algunas de sus subcarpetas, sin pasar el contenido por el contexto del agente.

Uso: python bajar-de-drive.py <id_carpeta> <destino> [subcarpeta ...]
  <id_carpeta>: el enlace compartido que el dueño pasa en el prompt, o solo su id (nunca va al repositorio).
  subcarpeta:   ruta relativa a la carpeta, por ejemplo  pruebas/base  pruebas/hojas-de-respuestas  caso-ps
                Sin subcarpetas, baja todo.
Deja los archivos en <destino>/datos/ y la lista en <destino>/lista.tsv (id, ruta, bytes). No imprime nombres.
Si bajó pruebas/base/, comprueba la huella de la base contra su ficha.
Necesita que el entorno permita drive.google.com y drive.usercontent.google.com.
"""
import hashlib
import html
import json
import os
import re
import subprocess
import sys
import urllib.request

sys.stdout.reconfigure(encoding="utf-8")
ENTRY = re.compile(
    r'<div class="flip-entry" id="entry-([A-Za-z0-9_-]+)".*?'
    r'href="https://drive\.google\.com/(file/d|drive/folders)/.*?'
    r'<div class="flip-entry-title">(.*?)</div>',
    re.S,
)


def listar(folder_id):
    url = f"https://drive.google.com/embeddedfolderview?id={folder_id}"
    with urllib.request.urlopen(url, timeout=60) as r:
        h = r.read().decode("utf-8")
    for fid, kind, title in ENTRY.findall(h):
        nombre = html.unescape(title).replace("/", "_").replace("\\", "_")
        yield fid, kind == "drive/folders", "_" if nombre in ("", ".", "..") else nombre


def sirve(rel, pedidas):
    """Si hay que entrar a la carpeta o bajar el archivo `rel` para las subcarpetas pedidas."""
    if not pedidas:
        return True
    return any(rel == p or rel.startswith(p + "/") or p.startswith(rel + "/") for p in pedidas)


def recorrer(folder_id, pedidas, ruta=""):
    for fid, es_carpeta, nombre in listar(folder_id):
        rel = f"{ruta}/{nombre}" if ruta else nombre
        if not sirve(rel, pedidas):
            continue
        if es_carpeta:
            yield from recorrer(fid, pedidas, rel)
        elif not pedidas or any(rel.startswith(p + "/") or rel == p for p in pedidas):
            yield fid, rel


def sha256(path):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for block in iter(lambda: f.read(1 << 20), b""):
            h.update(block)
    return h.hexdigest()


def main():
    raiz, destino = sys.argv[1], sys.argv[2]
    m = re.search(r"folders/([A-Za-z0-9_-]+)|[?&]id=([A-Za-z0-9_-]+)", raiz)
    if m:
        raiz = m.group(1) or m.group(2)
    pedidas = [p.strip("/").replace("\\", "/") for p in sys.argv[3:]]
    datos = os.path.join(destino, "datos")
    archivos = errores = total = 0
    os.makedirs(destino, exist_ok=True)
    with open(os.path.join(destino, "lista.tsv"), "w", encoding="utf-8") as out:
        for fid, rel in recorrer(raiz, pedidas):
            path = os.path.join(datos, *rel.split("/"))
            os.makedirs(os.path.dirname(path), exist_ok=True)
            url = f"https://drive.usercontent.google.com/download?id={fid}&export=download&confirm=t"
            r = subprocess.run(["curl", "-sS", "-L", "-o", path, "-w", "%{http_code}", url], capture_output=True, text=True)
            with open(path, "rb") as f:
                pagina = f.read(15).lower().startswith((b"<!doctype html", b"<html"))
            if r.stdout != "200" or (pagina and not rel.lower().endswith((".html", ".htm"))):
                errores += 1
                os.remove(path)
                continue
            n = os.path.getsize(path)
            total += n
            archivos += 1
            out.write(f"{fid}\t{rel}\t{n}\n")
    print(f"archivos={archivos} bytes={total} errores={errores}")
    ficha = os.path.join(datos, "pruebas", "base", "ficha.json")
    if os.path.exists(ficha):
        f = json.load(open(ficha, encoding="utf-8"))
        zpath = os.path.join(os.path.dirname(ficha), f["archivo"])
        ok = os.path.exists(zpath) and sha256(zpath) == f["sha256"]
        print(f"base del {f['fecha']}, versión {f['version_de_la_base']}: huella {'ok' if ok else 'NO COINCIDE: no usarla'}")


if __name__ == "__main__":
    main()
