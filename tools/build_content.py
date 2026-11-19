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
VERSION = "0.2.2"
# The Horizon build the mod is made for, and the horizon-mixins tool its `[native]` names.
HORIZON = "0b14f8acfd97fa2c750e42795249dcc981178ca4"
MIXINS = "e59da9094f0c2fdda6c627fe9eff22dec6a906f0"

# name, rows, english, portuguese, rarity, (body, dark, trim), upgrade material from the tier below
TIERS = [
    ("leather_backpack", 2, "Leather Backpack", "Mochila de Couro", "common", (0x9A, 0x5B, 0x2E), None),
    ("iron_backpack", 3, "Iron Backpack", "Mochila de Ferro", "common", (0xC8, 0xCC, 0xD0), "minecraft:iron_block"),
    ("gold_backpack", 4, "Gold Backpack", "Mochila de Ouro", "uncommon", (0xF2, 0xC6, 0x3A), "minecraft:gold_block"),
    ("diamond_backpack", 5, "Diamond Backpack", "Mochila de Diamante", "rare", (0x5C, 0xE0, 0xD8), "minecraft:diamond_block"),
    ("netherite_backpack", 6, "Netherite Backpack", "Mochila de Netherita", "epic", (0x4A, 0x42, 0x48), "minecraft:netherite_ingot"),
]
# Vast Depths' metals, past a chest menu's 54 slots: pages of 45 (the last row turns them). Their recipes are there
# only while Vast Depths is installed.
# name, pages, english, portuguese, trim, upgrade material
VAST = [
    ("stellarite_backpack", 2, "Stellarite Backpack", "Mochila de Estelarita", (0xF0, 0xB3, 0x3A), "vastdepths:stellarite_block"),
    ("infernium_backpack", 3, "Infernium Backpack", "Mochila de Infernium", (0xE0, 0x56, 0x2A), "vastdepths:infernium_block"),
    ("voidium_backpack", 4, "Voidium Backpack", "Mochila de Voidium", (0xC2, 0x3C, 0xE6), "vastdepths:voidium_block"),
]
PER_PAGE = 45

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
    lang_en = {"vastpacks.slots": "%s slots", "vastpacks.pages": "%s pages", "vastpacks.previous": "Previous page",
               "vastpacks.next": "Next page", "vastpacks.page": "Page %s of %s", "vastpacks.title_paged": "%s (%s/%s)", "vastpacks.busy": "That backpack is already open.", "vastpacks.legacy": "This backpack still holds its Vast Packs 0.1 contents in the world. An operator must open it once to move them into it.",
               "vastpacks.no_world": "Backpacks open in a world."}
    lang_pt = {"vastpacks.slots": "%s espaços", "vastpacks.pages": "%s páginas", "vastpacks.previous": "Página anterior",
               "vastpacks.next": "Próxima página", "vastpacks.page": "Página %s de %s", "vastpacks.title_paged": "%s (%s/%s)", "vastpacks.busy": "Essa mochila já está aberta.", "vastpacks.legacy": "Esta mochila ainda guarda o conteúdo da Vast Packs 0.1 no mundo. Um operador precisa abri-la uma vez para trazê-lo para dentro dela.",
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
    for name, pages, en, pt, trim, material in VAST:
        full = f"{NS}:{name}"
        slots = pages * PER_PAGE
        files[f"data/{NS}/horizon/item/{name}.json"] = {
            "template": "minecraft:leather", "max_stack_size": 1, "rarity": "epic",
            "components": {"minecraft:lore": [
                {"translate": "vastpacks.slots", "with": [str(slots)], "color": "gray", "italic": False},
                {"translate": "vastpacks.pages", "with": [str(pages)], "color": "dark_gray", "italic": False}]}}
        files[f"assets/{NS}/models/item/{name}.json"] = {"parent": "minecraft:item/generated",
                                                         "textures": {"layer0": f"{NS}:item/{name}"}}
        files[f"assets/{NS}/items/{name}.json"] = {"model": {"type": "minecraft:model", "model": f"{NS}:item/{name}"}}
        files[f"assets/{NS}/textures/item/{name}.png"] = png(sprite(trim))
        lang_en[f"item.{NS}.{name}"], lang_pt[f"item.{NS}.{name}"] = en, pt
        files[f"data/{NS}/recipe/{name}.json"] = {
            "type": "minecraft:crafting_transmute", "category": "equipment", "group": "vastpacks_upgrade",
            "input": below, "material": material, "result": {"id": full},
            "neoforge:conditions": [{"type": "neoforge:mod_loaded", "modid": "vastdepths"}]}
        items.append(full)
        below = full
    files[f"data/{NS}/tags/item/backpacks.json"] = {"values": items}
    files[f"data/{NS}/horizon/creative_tab/main.json"] = {"title": "Vast Packs", "icon": f"{NS}:diamond_backpack", "items": items}
    files[f"assets/{NS}/lang/en_us.json"] = dict(sorted(lang_en.items()))
    files[f"assets/{NS}/lang/pt_br.json"] = dict(sorted(lang_pt.items()))

    out = Path(out)
    for sub in ("assets", "data"):
        if (out / sub).exists():
            shutil.rmtree(out / sub)
    for path, value in files.items():
        target = out / path
        target.parent.mkdir(parents=True, exist_ok=True)
        if isinstance(value, bytes):
            target.write_bytes(value)
        else:
            target.write_text(json.dumps(value, indent=2, ensure_ascii=False) + "\n")
    (out / "horizon-mod.toml").write_text(
        f'id = "{NS}"\nname = "Vast Packs"\nversion = "{VERSION}"\n'
        'description = "Backpacks: right-click to open; what they hold travels with them."\n'
        'authors = ["voltzz900"]\napi = "0.2"\nside = "both"\ncapabilities = ["registries"]\n\n'
        f'[native]\ngame = "26.3"\nhorizon = "{HORIZON}"\nmixins = "{MIXINS}"\n')
    print(f"{NS}: {len(items)} backpacks, {len(files) + 1} files")


if __name__ == "__main__":
    build(Path(__file__).resolve().parent.parent / "mod")
