"""Makes the small copies of the PNG masters that the interface draws (UI-013).

The masters in assets/ are 1254 px square. Drawn from those, twenty rows of icons held a drag
at 20 frames a second; from 128 px copies, at 60 (measured by the owner, 2026-09-27). Each copy
is twice the largest size the interface draws it, so it stays sharp at 200 % scaling. The
copies are committed; run this only after a master changes:

    python tools/uiicons.py
"""

from pathlib import Path

from PIL import Image

ROOT = Path(__file__).resolve().parent.parent
MASTERS = ROOT / "assets"
COPIES = MASTERS / "ui"
# Every icon is drawn at 49 px or less (a toolbar or row button, 68 px at 72 %), twice that
# rounded up to a power of two.
DEFAULT_SIZE = 128
# The application icon is also setup's mark, drawn at 126 px.
SIZES = {"application-icon.png": 256}


def main() -> None:
    COPIES.mkdir(exist_ok=True)
    for master in sorted(MASTERS.glob("*.png")):
        size = SIZES.get(master.name, DEFAULT_SIZE)
        with Image.open(master) as image:
            copy = image.convert("RGBA")
            copy.thumbnail((size, size), Image.LANCZOS)
            copy.save(COPIES / master.name, optimize=True)
        print(f"wrote {(COPIES / master.name).relative_to(ROOT)}: {copy.size[0]}x{copy.size[1]}")


if __name__ == "__main__":
    main()
