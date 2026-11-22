"""Sanity-check icon images and sizes; standard library only."""
from pathlib import Path
import struct

ROOT = Path(__file__).resolve().parent.parent
ASSETS = ROOT / "assets"
MSIX = ROOT / "packaging" / "msix" / "Assets"
ICON_SIZES = [16, 20, 24, 32, 40, 48, 64, 96, 128, 256]
TARGET_SIZES = [16, 20, 24, 30, 32, 36, 40, 48, 60, 64, 72, 80, 96, 256]


def png_dimensions(data):
    assert data[:8] == b"\x89PNG\r\n\x1a\n", "Expected PNG"
    assert data[12:16] == b"IHDR", "Expected PNG IHDR"
    return struct.unpack_from(">II", data, 16)


def ico_frames(path):
    data = path.read_bytes()
    reserved, kind, count = struct.unpack_from("<HHH", data, 0)
    assert (reserved, kind, count) == (0, 1, len(ICON_SIZES))
    frames = {}
    for i in range(count):
        w, h, _, _, planes, bpp, length, offset = struct.unpack_from(
            "<BBBBHHII", data, 6 + i * 16)
        size = w or 256
        assert h == w and planes == 1 and bpp == 32
        frame = data[offset:offset + length]
        assert len(frame) == length
        assert png_dimensions(frame) == (size, size)
        frames[size] = frame
    assert sorted(frames) == ICON_SIZES
    return frames


def verify():
    app = ico_frames(ASSETS / "SwiftTray.ico")
    tray = ico_frames(ASSETS / "SwiftTray-tray.ico")
    # Visual identity is the same, but the tray is purposefully framed 8%
    # larger inside the transparent ICO canvas. They MUST NOT have identical
    # PNG frames; that was the old pre-tuning baseline.
    for size in ICON_SIZES:
        assert app[size] != tray[size], f"Tray optical zoom missing at {size}px"
    for size in (16, 20, 24, 32):
        assert tray[size] == (ASSETS / f"SwiftTray-tray-{size}.png").read_bytes(), (
            f"Independent tray PNG does not match embedded ICO frame ({size}px)")
    for name, base in (("Square44x44Logo", 44),
                       ("Square150x150Logo", 150),
                       ("StoreLogo", 50)):
        assert png_dimensions((MSIX / f"{name}.png").read_bytes()) == (base, base)
        for scale in (100, 125, 150, 200, 400):
            size = (base * scale + 50) // 100
            assert png_dimensions((MSIX / f"{name}.scale-{scale}.png").read_bytes()) == (size, size)
    for size in TARGET_SIZES:
        for suffix in ("", "_altform-unplated"):
            name = f"Square44x44Logo.targetsize-{size}{suffix}.png"
            assert png_dimensions((MSIX / name).read_bytes()) == (size, size)
    print("PASS: app icon unchanged, optical tray zoom in 10 frames, PNG mirrors, 46 MSIX images sized correctly.")


if __name__ == "__main__":
    verify()
