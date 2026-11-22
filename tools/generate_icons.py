"""Regenerate SwiftTray unified icon assets for Windows and optional MSIX.

App and notification area use the same symbol and palette.
The tray-only ICO is optically enlarged by 8% in its own transparent canvas;
app/Explorer/Start/About/MSIX image resources keep their original proportions.
The earlier four-panel mark and prior unified render are retained as rollback assets.

Designer tooling only; building SwiftTray does not require Python.
Install dependencies with: pip install pillow cairosvg
Run from any directory: python tools/generate_icons.py
"""
from pathlib import Path
from PIL import Image
import cairosvg
import io
import struct

ROOT = Path(__file__).resolve().parent.parent
ASSETS = ROOT / "assets"
MSIX = ROOT / "packaging" / "msix" / "Assets"
ICON_SIZES = [16, 20, 24, 32, 40, 48, 64, 96, 128, 256]
MSIX_SCALES = [100, 125, 150, 200, 400]
MSIX_TARGET_SIZES = [16, 20, 24, 30, 32, 36, 40, 48, 60, 64, 72, 80, 96, 256]
# Maximum safe optical zoom of this particular approved mark before vertical clipping.
TRAY_ZOOM = 1.08


def render(svg: bytes, size: int) -> Image.Image:
    """Rasterize from the approved transparent master at native icon sizes."""
    scale = 2 if size <= 256 else 1
    png = cairosvg.svg2png(bytestring=svg, output_width=size * scale,
                          output_height=size * scale)
    return Image.open(io.BytesIO(png)).convert("RGBA").resize(
        (size, size), Image.Resampling.LANCZOS)


def save_ico(path: Path, images: list[Image.Image]) -> None:
    """Encode per-size PNG frames into a Windows multi-resolution ICO."""
    payloads = []
    for image in images:
        blob = io.BytesIO()
        image.save(blob, format="PNG", optimize=True)
        payloads.append(blob.getvalue())
    offset = 6 + len(images) * 16
    with path.open("wb") as output:
        output.write(struct.pack("<HHH", 0, 1, len(images)))
        for image, payload in zip(images, payloads):
            width, height = image.size
            assert width == height and width in ICON_SIZES
            output.write(struct.pack("<BBBBHHII", width if width < 256 else 0,
                                     height if height < 256 else 0,
                                     0, 0, 1, 32, len(payload), offset))
            offset += len(payload)
        for payload in payloads:
            output.write(payload)


def render_tray(svg: bytes, size: int) -> Image.Image:
    """Optically enlarge the *same* approved symbol in the tray ICO only.

    The refined master has ~4% vertical transparent margins; 1.08x zoom
    leaves a little safety margin and does not clip visible artwork. We crop
    from the full-resolution approved source before antialiased downsampling
    so 16/20/24px Windows Shell icons remain crisp.
    """
    # 698px equals the artwork's native SVG viewBox / embedded raster master.
    master = render(svg, 698)
    side = int(round(master.width / TRAY_ZOOM))
    left = (master.width - side) // 2
    top = (master.height - side) // 2
    crop = master.crop((left, top, left + side, top + side))
    return crop.resize((size, size), Image.Resampling.LANCZOS)


def build_icon() -> None:
    original = (ASSETS / "SwiftTray-logo.svg").read_bytes()

    # One visual identity for EXE, Explorer, desktop and Windows taskbar.
    save_ico(ASSETS / "SwiftTray.ico", [render(original, s) for s in ICON_SIZES])

    # Win32 resource ID 2 is tray-only, same shape with 8% tighter framing.
    tray_frames = {size: render_tray(original, size) for size in ICON_SIZES}
    save_ico(ASSETS / "SwiftTray-tray.ico",
             [tray_frames[size] for size in ICON_SIZES])
    for size in (16, 20, 24, 32):
        tray_frames[size].save(ASSETS / f"SwiftTray-tray-{size}.png", optimize=True)

    render(original, 512).save(ASSETS / "SwiftTray-preview.png", optimize=True)
    about = render(original, 256)
    about.save(ASSETS / "SwiftTray-transparent.png", optimize=True)
    # GDI+ PixelFormat32bppPARGB expects premultiplied BGRA (little-endian).
    pixels = bytearray()
    for r, g, b, alpha in about.getdata():
        pixels.extend(((b * alpha + 127) // 255,
                       (g * alpha + 127) // 255,
                       (r * alpha + 127) // 255, alpha))
    assert len(pixels) == 256 * 256 * 4
    (ASSETS / "SwiftTray-about.pbgra").write_bytes(pixels)

    # A reusable MSIX asset set, not a substitute for a real AppxManifest.xml.
    # In MSIX, taskbar and Start icons are taken from these PNG resources,
    # which do NOT automatically inherit the executable's embedded ICO.
    MSIX.mkdir(parents=True, exist_ok=True)
    for name, base_size in (("Square44x44Logo", 44),
                            ("Square150x150Logo", 150),
                            ("StoreLogo", 50)):
        render(original, base_size).save(MSIX / f"{name}.png", optimize=True)
        for scale in MSIX_SCALES:
            scaled = (base_size * scale + 50) // 100
            render(original, scaled).save(
                MSIX / f"{name}.scale-{scale}.png", optimize=True)

    for size in MSIX_TARGET_SIZES:
        icon = render(original, size)
        icon.save(MSIX / f"Square44x44Logo.targetsize-{size}.png", optimize=True)
        icon.save(MSIX / f"Square44x44Logo.targetsize-{size}_altform-unplated.png",
                  optimize=True)

    print("Generated refined app icon, +8% tray optical-size icon (same symbol), and MSIX PNG resources.")


if __name__ == "__main__":
    build_icon()
