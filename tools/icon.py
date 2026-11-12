#!/usr/bin/env python3
"""Draws icons/vastpacks.png (256x256): the diamond backpack on a dark tile."""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from build_content import TIERS, png, sprite  # noqa: E402


def main():
    bg = (0x1B, 0x1E, 0x24, 255)
    s = sprite(TIERS[3][5])
    px = [[s[y // 16][x // 16] if s[y // 16][x // 16][3] else bg for x in range(256)] for y in range(256)]
    out = Path(__file__).resolve().parent.parent / "icons/vastpacks.png"
    out.write_bytes(png(px))
    print(out)


if __name__ == "__main__":
    main()
