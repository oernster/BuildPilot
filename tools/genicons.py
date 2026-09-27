"""Makes assets/application-icon.ico from assets/application-icon.png (INST-006).

The .ico is committed; run this only after the PNG master changes:

    python tools/genicons.py
"""

from pathlib import Path

from PIL import Image

ROOT = Path(__file__).resolve().parent.parent
MASTER = ROOT / "assets" / "application-icon.png"
ICON = ROOT / "assets" / "application-icon.ico"
# Every size Windows asks an icon for: lists, taskbar, Start, Explorer's large views.
SIZES = [(16, 16), (20, 20), (24, 24), (32, 32), (40, 40), (48, 48), (64, 64), (128, 128), (256, 256)]


def main() -> None:
    master = Image.open(MASTER).convert("RGBA")
    master.save(ICON, format="ICO", sizes=SIZES)
    written = Image.open(ICON)
    print(f"wrote {ICON.relative_to(ROOT)}: {sorted(written.info.get('sizes', set()))}")


if __name__ == "__main__":
    main()
