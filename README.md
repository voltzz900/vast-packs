# Vast Packs

Backpacks for [Horizon](https://playhorizon.dev) (Minecraft 26.3 rewritten in Rust): right-click one in your main
hand and it opens like a chest.

| Backpack | Slots | Made from |
| --- | --- | --- |
| Leather Backpack | 18 | leather, string and a chest |
| Iron Backpack | 27 | a Leather Backpack and a block of iron |
| Gold Backpack | 36 | an Iron Backpack and a block of gold |
| Diamond Backpack | 45 | a Gold Backpack and a block of diamond |
| Netherite Backpack | 54 | a Diamond Backpack and a netherite ingot |
| Stellarite Backpack | 90 (2 pages) | a Netherite Backpack and a block of Stellarite |
| Infernium Backpack | 135 (3 pages) | a Stellarite Backpack and a block of Infernium |
| Voidium Backpack | 180 (4 pages) | an Infernium Backpack and a block of Voidium |

An upgrade (in the crafting grid, the backpack and the material) keeps everything the backpack holds. The last three
need [Vast Depths](https://github.com/voltzz900/vast-ores): their recipes are there only while it is installed. A
chest menu holds at most 54 slots, so they show 45 a page and their last row turns the pages.

## How it keeps your things

What a backpack holds is in the backpack itself (its container, as a shulker box's), so it travels with it: dropped,
traded, kept in a chest. Its menu's slots are the backpack's own, through Horizon's mod API:

- every move goes through the backpack's storage in one transaction, so nothing can be counted twice;
- the backpack in your hand can't go into itself, and the menu closes once it leaves your hand;
- an upgrade keeps everything it holds.

Names in English and Brazilian Portuguese. The menu is a plain chest menu, so it should work on servers too.

**Install** `vastpacks-<version>.hmod` (one package: the code, items, art, recipes and names), then build natives in
the launcher.

**From 0.1:** 0.1 kept a backpack's contents in the world (`<world>/vastpacks/<id>.dat`) and came as two files
(`vastpacks_native-0.1.0.hnmod`, which launchers from 0.2.70 no longer load, and `vastpacks-0.1.0.hmod`). The
file names no owner, so the contents move only while an **operator** holds the backpack: an operator opens a 0.1
backpack once with 0.2.2 and its contents move from that file into it; anyone else is told to ask one. The moved
backpack loses its 0.1 id, so it is never imported twice, and the file is renamed `<id>.dat.moved` and kept. If the
game stops before the backpack is saved, it still has the id and is empty, and the next open fills it again from
that file.

## Not yet

Upgrades (pickup, magnet, feeding, restock), opening with a key, wearing it.

## Building

The mod is `mod/`, a crate on Horizon's mod API. `python3 tools/package.py` writes `dist/vastpacks-<v>.hmod`
with `cargo horizon-mod build` (`tools/build_content.py` makes the content into `mod/`, with every sprite drawn in
it; `tools/icon.py` the icon).

## License

MIT, see [LICENSE](LICENSE). Inspired by Sophisticated Backpacks; no code, art or text of it is used, and this mod
is not affiliated with its authors.
