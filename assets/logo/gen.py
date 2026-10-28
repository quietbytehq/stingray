"""Builds every Stingray icon from assets/logo/source.jpeg (the logo on a white background).

White becomes transparent, the ray is centred on a square canvas with a margin, then all Linux PNG
sizes, the Windows .ico and the macOS .icns are written into src-tauri/icons.
Run from the repo root: python3 assets/logo/gen.py
"""
from pathlib import Path
from PIL import Image
import numpy as np
from scipy import ndimage

ROOT = Path(__file__).resolve().parents[2]
SRC = ROOT / "assets/logo/source.jpeg"
OUT = ROOT / "src-tauri/icons"
STATIC = ROOT / "static"
MARGIN = 0.06

def cut_out(img: Image.Image) -> Image.Image:
    rgb = np.asarray(img.convert("RGB")).astype(np.float32) / 255.0
    # Coverage from distance to white: the logo is saturated, so its weakest channel is low.
    alpha = 1.0 - rgb.min(axis=2)
    alpha = np.clip((alpha - 0.04) / 0.90, 0.0, 1.0)
    # Light facets are pale too, so only soften pixels at the outline. The background is the
    # near-white region touching the border; anything more than 3 px inside it is fully opaque.
    near_white = rgb.min(axis=2) > 0.88
    labels, _ = ndimage.label(near_white)
    edge = np.unique(np.concatenate([labels[0], labels[-1], labels[:, 0], labels[:, -1]]))
    background = np.isin(labels, edge[edge > 0])
    band = ndimage.binary_dilation(background, iterations=3)
    alpha = np.where(band, alpha, 1.0)
    alpha = np.where(background & (alpha < 0.5), 0.0, alpha)
    safe = np.maximum(alpha, 1e-3)[..., None]
    colour = np.clip((rgb - (1.0 - safe)) / safe, 0.0, 1.0)
    out = np.dstack([colour, alpha]) * 255.0
    return Image.fromarray(out.round().astype(np.uint8), "RGBA")

def squared(img: Image.Image) -> Image.Image:
    box = img.getchannel("A").point(lambda a: 255 if a > 8 else 0).getbbox()
    img = img.crop(box)
    side = int(max(img.size) / (1 - 2 * MARGIN))
    canvas = Image.new("RGBA", (side, side), (0, 0, 0, 0))
    canvas.paste(img, ((side - img.width) // 2, (side - img.height) // 2), img)
    return canvas

def main() -> None:
    OUT.mkdir(parents=True, exist_ok=True)
    master = squared(cut_out(Image.open(SRC))).resize((1024, 1024), Image.LANCZOS)
    master.save(ROOT / "assets/logo/stingray-1024.png")
    for size in (16, 24, 32, 48, 64, 128, 256, 512):
        master.resize((size, size), Image.LANCZOS).save(OUT / f"{size}x{size}.png")
    master.resize((256, 256), Image.LANCZOS).save(OUT / "128x128@2x.png")
    master.save(OUT / "icon.png")
    master.save(OUT / "icon.ico", sizes=[(16, 16), (24, 24), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)])
    master.save(OUT / "icon.icns")
    STATIC.mkdir(exist_ok=True)
    master.resize((512, 512), Image.LANCZOS).save(STATIC / "stingray.png")
    master.resize((64, 64), Image.LANCZOS).save(STATIC / "favicon.png")

if __name__ == "__main__":
    main()
