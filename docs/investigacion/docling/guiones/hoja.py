"""Contact sheet from rendered page PNGs. Usage: python -I hoja.py <out.png> <width> <png>..."""
import sys
from PIL import Image, ImageDraw
out, W = sys.argv[1], int(sys.argv[2]); ims = []
for p in sys.argv[3:]:
    im = Image.open(p).convert("L"); im.thumbnail((W, W * 2)); ims.append((p.split("/")[-1], im))
cols = 4; rows = (len(ims) + cols - 1) // cols; H = max(i.height for _, i in ims) + 20
sheet = Image.new("L", (cols * W, rows * H), 255); d = ImageDraw.Draw(sheet)
for k, (n, im) in enumerate(ims):
    x, y = (k % cols) * W, (k // cols) * H; sheet.paste(im, (x, y + 20)); d.text((x + 4, y + 4), n, fill=0)
sheet.save(out)
