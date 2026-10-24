"""Builds the square source of the app icon: the isotipo of Micelya centred
on a white square with rounded corners. Pure Python (no PIL): decodes the
RGBA PNG, scales it with bilinear filtering on premultiplied colour, and
writes a new PNG. Usage: make_icon.py <isotipo.png> <out.png> [size]"""
import struct
import sys
import zlib


def read_png(path):
    data = open(path, "rb").read()
    assert data[:8] == b"\x89PNG\r\n\x1a\n"
    pos = 8
    idat = b""
    while pos < len(data):
        length = struct.unpack(">I", data[pos:pos + 4])[0]
        kind = data[pos + 4:pos + 8]
        body = data[pos + 8:pos + 8 + length]
        if kind == b"IHDR":
            width, height, depth, ctype, _, _, interlace = struct.unpack(">IIBBBBB", body)
            assert depth == 8 and ctype == 6 and interlace == 0, (depth, ctype, interlace)
        elif kind == b"IDAT":
            idat += body
        pos += 12 + length
    raw = zlib.decompress(idat)
    bpp = 4
    stride = width * bpp
    rows = []
    prev = bytearray(stride)
    i = 0
    for _ in range(height):
        ftype = raw[i]
        line = bytearray(raw[i + 1:i + 1 + stride])
        i += 1 + stride
        for x in range(stride):
            a = line[x - bpp] if x >= bpp else 0
            b = prev[x]
            c = prev[x - bpp] if x >= bpp else 0
            if ftype == 1:
                line[x] = (line[x] + a) & 255
            elif ftype == 2:
                line[x] = (line[x] + b) & 255
            elif ftype == 3:
                line[x] = (line[x] + ((a + b) >> 1)) & 255
            elif ftype == 4:
                p = a + b - c
                pa, pb, pc = abs(p - a), abs(p - b), abs(p - c)
                pred = a if pa <= pb and pa <= pc else (b if pb <= pc else c)
                line[x] = (line[x] + pred) & 255
        rows.append(line)
        prev = line
    # premultiplied floats
    pixels = []
    for line in rows:
        row = []
        for x in range(width):
            r, g, b, al = line[x * 4:x * 4 + 4]
            f = al / 255.0
            row.append((r * f, g * f, b * f, al))
        pixels.append(row)
    return width, height, pixels


def sample(pixels, w, h, fx, fy):
    x0 = int(fx)
    y0 = int(fy)
    dx = fx - x0
    dy = fy - y0

    def px(x, y):
        if 0 <= x < w and 0 <= y < h:
            return pixels[y][x]
        return (0.0, 0.0, 0.0, 0.0)

    out = [0.0, 0.0, 0.0, 0.0]
    for (xx, yy, wt) in ((x0, y0, (1 - dx) * (1 - dy)), (x0 + 1, y0, dx * (1 - dy)), (x0, y0 + 1, (1 - dx) * dy), (x0 + 1, y0 + 1, dx * dy)):
        p = px(xx, yy)
        for k in range(4):
            out[k] += p[k] * wt
    return out


def rounded_coverage(x, y, size, radius, inset):
    # 4x4 supersampling of a rounded square [inset, size - inset]
    hits = 0
    lo = inset
    hi = size - inset
    for sy in range(4):
        for sx in range(4):
            px = x + (sx + 0.5) / 4
            py = y + (sy + 0.5) / 4
            if px < lo or px > hi or py < lo or py > hi:
                continue
            cx = min(max(px, lo + radius), hi - radius)
            cy = min(max(py, lo + radius), hi - radius)
            if (px - cx) ** 2 + (py - cy) ** 2 <= radius ** 2:
                hits += 1
    return hits / 16.0


def write_png(path, size, rgba_rows):
    raw = b"".join(b"\x00" + bytes(row) for row in rgba_rows)

    def chunk(kind, body):
        return struct.pack(">I", len(body)) + kind + body + struct.pack(">I", zlib.crc32(kind + body) & 0xFFFFFFFF)

    png = b"\x89PNG\r\n\x1a\n"
    png += chunk(b"IHDR", struct.pack(">IIBBBBB", size, size, 8, 6, 0, 0, 0))
    png += chunk(b"IDAT", zlib.compress(raw, 9))
    png += chunk(b"IEND", b"")
    open(path, "wb").write(png)


def main():
    src, out = sys.argv[1], sys.argv[2]
    size = int(sys.argv[3]) if len(sys.argv) > 3 else 512
    w, h, pixels = read_png(src)
    inset = size * 0.04
    radius = size * 0.22
    # the isotipo takes 72% of the width, centred, a little lower for its arc
    target_w = size * 0.72
    scale = target_w / w
    target_h = h * scale
    ox = (size - target_w) / 2
    oy = (size - target_h) / 2 + size * 0.01
    rows = []
    for y in range(size):
        row = bytearray(size * 4)
        for x in range(size):
            cov = rounded_coverage(x, y, size, radius, inset)
            # background: white, premultiplied
            br, bg, bb, ba = 255.0 * cov, 255.0 * cov, 255.0 * cov, 255.0 * cov
            fx = (x + 0.5 - ox) / scale - 0.5
            fy = (y + 0.5 - oy) / scale - 0.5
            if -1 <= fx <= w and -1 <= fy <= h:
                r, g, b, a = sample(pixels, w, h, fx, fy)
                fa = a / 255.0
                br = r + br * (1 - fa)
                bg = g + bg * (1 - fa)
                bb = b + bb * (1 - fa)
                ba = a + ba * (1 - fa)
            if ba > 0:
                f = 255.0 / ba
                row[x * 4:x * 4 + 4] = bytes((min(255, round(br * f)), min(255, round(bg * f)), min(255, round(bb * f)), min(255, round(ba))))
        rows.append(row)
    write_png(out, size, rows)


main()
