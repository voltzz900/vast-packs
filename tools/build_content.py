#!/usr/bin/env python3
"""Builds the content half of Vast Packs (`vastpacks`): the backpacks as items, their sprites (drawn here), recipes
and names. The native mod `vastpacks_native` opens them and keeps what they hold.

Usage: build_content.py <output dir>   (writes <output dir>/vastpacks)
"""
import json
import shutil
import struct
import sys
import zlib
from pathlib import Path

NS = "vastpacks"
VERSION = "0.1.0"

# name, rows, english, portuguese, rarity, (body, dark, trim), upgrade material from the tier below
TIERS = [
    ("leather_backpack", 2, "Leather Backpack", "Mochila de Couro", "common", (0x9A, 0x5B, 0x2E), None),
    ("iron_backpack", 3, "Iron Backpack", "Mochila de Ferro", "common", (0xC8, 0xCC, 0xD0), "minecraft:iron_block"),
    ("gold_backpack", 4, "Gold Backpack", "Mochila de Ouro", "uncommon", (0xF2, 0xC6, 0x3A), "minecraft:gold_block"),
    ("diamond_backpack", 5, "Diamond Backpack", "Mochila de Diamante", "rare", (0x5C, 0xE0, 0xD8), "minecraft:diamond_block"),
    ("netherite_backpack", 6, "Netherite Backpack", "Mochila de Netherita", "epic", (0x4A, 0x42, 0x48), "minecraft:netherite_ingot"),
]

LEATHER = (0x8A, 0x52, 0x2A)


def shade(c, k):
    return tuple(max(0, min(255, round(v * k))) for v in c)


def sprite(trim):
    """A backpack seen from the front: a leather body, a flap with the tier's metal on its edge, a buckle, straps."""
    clear = (0, 0, 0, 0)
    px = [[clear] * 16 for _ in range(16)]
    outline, body, dark, light = (0x2A, 0x18, 0x0C), LEATHER, shade(LEATHER, 0.72), shade(LEATHER, 1.22)
    metal, metal_dark, metal_light = trim, shade(trim, 0.62), shade(trim, 1.25)

    def put(x, y, c):
        px[y][x] = (*c, 255)

    # The carrying handle on top.
    for x in range(6, 10):
        put(x, 1, outline)
    put(5, 2, outline)
    put(10, 2, outline)
    # The body.
    for y in range(3, 15):
        for x in range(3, 13):
            edge = x in (3, 12) or y in (3, 14)
            put(x, y, outline if edge else (light if x == 4 else dark if x == 11 or y == 13 else body))
    # The side pockets.
    for y in range(8, 13):
        put(2, y, outline)
        put(13, y, outline)
    # The flap, edged with the tier's metal.
    for y in range(4, 8):
        for x in range(4, 12):
            put(x, y, metal_light if y == 4 else metal if y == 7 else shade(LEATHER, 0.9))
    for x in range(4, 12):
        put(x, 8, metal_dark)
    # The buckle.
    for x, y in ((7, 8), (8, 8), (7, 9), (8, 9)):
        put(x, y, metal_light if y == 8 else metal)
    put(7, 10, metal_dark)
    put(8, 10, metal_dark)
    # A pocket seam.
    for x in range(5, 11):
        put(x, 12, dark)
    return px


def png(pixels):
    raw = b"".join(b"\x00" + bytes(c for p in row for c in p) for row in pixels)

    def chunk(kind, data):
        return struct.pack(">I", len(data)) + kind + data + struct.pack(">I", zlib.crc32(kind + data) & 0xFFFFFFFF)

    return (b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", len(pixels[0]), len(pixels), 8, 6, 0, 0, 0))
            + chunk(b"IDAT", zlib.compress(raw, 9)) + chunk(b"IEND", b""))


def build(out):
    files, items = {}, []
    lang_en = {"vastpacks.slots": "%s slots", "vastpacks.busy": "That backpack is already open.",
               "vastpacks.no_world": "Backpacks open in a world."}
    lang_pt = {"vastpacks.slots": "%s espaços", "vastpacks.busy": "Essa mochila já está aberta.",
               "vastpacks.no_world": "Mochilas abrem dentro de um mundo."}
    below = None
    for name, rows, en, pt, rarity, trim, material in TIERS:
        full = f"{NS}:{name}"
        files[f"data/{NS}/horizon/item/{name}.json"] = {
            "template": "minecraft:leather", "max_stack_size": 1, "rarity": rarity,
            "components": {"minecraft:lore": [{"translate": "vastpacks.slots", "with": [str(rows * 9)],
                                               "color": "gray", "italic": False}]}}
        files[f"assets/{NS}/models/item/{name}.json"] = {"parent": "minecraft:item/generated",
                                                         "textures": {"layer0": f"{NS}:item/{name}"}}
        files[f"assets/{NS}/items/{name}.json"] = {"model": {"type": "minecraft:model", "model": f"{NS}:item/{name}"}}
        files[f"assets/{NS}/textures/item/{name}.png"] = png(sprite(trim))
        lang_en[f"item.{NS}.{name}"], lang_pt[f"item.{NS}.{name}"] = en, pt
        if below is None:
            files[f"data/{NS}/recipe/{name}.json"] = {
                "type": "minecraft:crafting_shaped", "category": "equipment",
                "key": {"L": "minecraft:leather", "S": "minecraft:string", "C": "minecraft:chest"},
                "pattern": ["SLS", "LCL", "LLL"], "result": {"id": full}}
        else:
            # The tier above keeps the backpack's components, and so its id and what it holds.
            files[f"data/{NS}/recipe/{name}.json"] = {
                "type": "minecraft:crafting_transmute", "category": "equipment", "group": "vastpacks_upgrade",
                "input": below, "material": material, "result": {"id": full}}
        items.append(full)
        below = full
    files[f"data/{NS}/tags/item/backpacks.json"] = {"values": items}
    files[f"data/{NS}/horizon/creative_tab/main.json"] = {"title": "Vast Packs", "icon": f"{NS}:diamond_backpack", "items": items}
    files[f"assets/{NS}/lang/en_us.json"] = dict(sorted(lang_en.items()))
    files[f"assets/{NS}/lang/pt_br.json"] = dict(sorted(lang_pt.items()))

    out = Path(out) / NS
    if out.exists():
        shutil.rmtree(out)
    for path, value in files.items():
        target = out / path
        target.parent.mkdir(parents=True, exist_ok=True)
        if isinstance(value, bytes):
            target.write_bytes(value)
        else:
            target.write_text(json.dumps(value, indent=2, ensure_ascii=False) + "\n")
    (out / "horizon-mod.toml").write_text(
        f'id = "{NS}"\nname = "Vast Packs (content)"\nversion = "{VERSION}"\n'
        'description = "The backpacks of Vast Packs: their items, art, recipes and names. Needs the native mod."\n'
        'authors = ["voltzz900"]\napi = "0.2"\nside = "both"\ncapabilities = ["registries"]\n')
    print(f"{NS}: {len(items)} backpacks, {len(files) + 1} files")


if __name__ == "__main__":
    build(sys.argv[1] if len(sys.argv) > 1 else Path(__file__).resolve().parent.parent / "build")
