"""Regenerate the multi-size Windows icon from the included SwiftTray SVGs.

Optional designer tooling only; building SwiftTray does NOT require Python.
Dependencies for regeneration: pip install pillow cairosvg
"""
from pathlib import Path
from PIL import Image
import cairosvg
import io
import struct

ASSETS = Path(__file__).resolve().parent.parent / "assets"
SIZES = [16, 20, 24, 32, 40, 48, 64, 96, 128, 256]


def render(svg: bytes, size: int) -> Image.Image:
    png = cairosvg.svg2png(bytestring=svg, output_width=size*4, output_height=size*4)
    return Image.open(io.BytesIO(png)).convert("RGBA").resize((size, size), Image.Resampling.LANCZOS)


def build_icon() -> None:
    tiny_16 = (ASSETS / "SwiftTray-tray-16.svg").read_bytes()
    small = (ASSETS / "SwiftTray-tray-small.svg").read_bytes()
    tray = (ASSETS / "SwiftTray-tray.svg").read_bytes()
    main = (ASSETS / "SwiftTray-logo.svg").read_bytes()
    entries = []
    payloads = []
    offset = 6 + len(SIZES)*16
    for size in SIZES:
        source = tiny_16 if size <= 16 else small if size <= 24 else tray if size <= 40 else main
        image = render(source, size)
        if size in (16,20,24,32):
            image.save(ASSETS / f"SwiftTray-tray-{size}.png", optimize=True)
        buffer=io.BytesIO()
        image.save(buffer,format="PNG",optimize=True)
        payload=buffer.getvalue()
        entries.append(struct.pack("<BBBBHHII",size if size<256 else 0,
                                   size if size<256 else 0,0,0,1,32,len(payload),offset))
        payloads.append(payload)
        offset+=len(payload)
    with (ASSETS / "SwiftTray.ico").open("wb") as out:
        out.write(struct.pack("<HHH",0,1,len(SIZES)))
        for entry in entries:out.write(entry)
        for payload in payloads:out.write(payload)
    render(main,512).save(ASSETS / "SwiftTray-preview.png",optimize=True)
    about = render(main,256)
    about.save(ASSETS / "SwiftTray-transparent.png",optimize=True)
    # GDI+ PixelFormat32bppPARGB expects premultiplied BGRA (little-endian).
    # This avoids HICON conversion entirely in the About panel.
    packed = bytearray()
    for r, g, b, alpha in about.getdata():
        packed.extend(((b * alpha + 127) // 255,
                       (g * alpha + 127) // 255,
                       (r * alpha + 127) // 255,
                       alpha))
    assert len(packed) == 256 * 256 * 4
    (ASSETS / "SwiftTray-about.pbgra").write_bytes(packed)
    print("Generated ICO, transparent previews and About PArgb bitmap.")


if __name__ == "__main__":
    build_icon()
