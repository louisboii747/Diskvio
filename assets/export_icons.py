"""Export the supplied artwork to native asset slots. Requires Pillow.

Run from any directory: python assets/export_icons.py
Source artwork is preserved; exports only resize and center it.
"""
from pathlib import Path
import json
import shutil
from PIL import Image

ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / "assets" / "Diskvio Icon.png"
MAC = ROOT / "macos/Diskvio/Diskvio/Assets.xcassets/AppIcon.appiconset"
WIN = ROOT / "windows/Diskvio.Windows/Assets"


def export(source, path, size, inset=1.0):
    side = round(min(size) * inset)
    icon = source.resize((side, side), Image.Resampling.LANCZOS)
    canvas = Image.new("RGBA", size)
    canvas.alpha_composite(icon, ((size[0] - side) // 2, (size[1] - side) // 2))
    canvas.save(path)


def main():
    source = Image.open(SOURCE).convert("RGBA")
    if source.width != source.height:
        raise ValueError("The source app icon must be square")
    catalog = json.loads((MAC / "Contents.json").read_text(encoding="utf-8"))
    for image in catalog["images"]:
        points = int(image["size"].split("x")[0])
        scale = int(image["scale"][0])
        image["filename"] = f"AppIcon-{points}@{scale}x.png"
        export(source, MAC / image["filename"], (points * scale,) * 2)
    (MAC / "Contents.json").write_text(json.dumps(catalog, indent=2) + "\n", encoding="utf-8", newline="\n")
    sizes = {
        "Square150x150Logo.scale-200.png": (300, 300),
        "Square44x44Logo.scale-200.png": (88, 88),
        "Square44x44Logo.targetsize-24_altform-unplated.png": (24, 24),
        "Square44x44Logo.targetsize-48_altform-lightunplated.png": (48, 48),
        "StoreLogo.png": (50, 50),
        "LockScreenLogo.scale-200.png": (48, 48),
        "SplashScreen.scale-200.png": (1240, 600),
        "Wide310x150Logo.scale-200.png": (620, 300),
    }
    for name, size in sizes.items():
        export(source, WIN / name, size, inset=0.7 if size[0] != size[1] else 1.0)
    shutil.copyfile(ROOT / "assets/diskvio-icon-windows.ico", WIN / "AppIcon.ico")
    print("Exported 10 macOS icon slots and 9 Windows assets.")


if __name__ == "__main__":
    main()
