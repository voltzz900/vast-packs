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

An upgrade (in the crafting grid, the backpack and the material) keeps everything the backpack holds.

## How it keeps your things

A backpack item carries only an id; what it holds is kept in the world, in `<world>/vastpacks/<id>.dat` (so world
backups carry it), the way Sophisticated Backpacks keeps it in the world's saved data. So:

- a backpack dropped, thrown or lost while it is open takes nothing with it, and nothing can be counted twice;
- a backpack put inside a backpack goes back to you when it closes (no backpack in a backpack);
- the contents are written when the menu closes, for any reason (closing it, leaving the world, dying); a new file
  is written and then renamed over the old one, and if it can't be written everything goes back to you.

Names in English and Brazilian Portuguese. It works on a server too: the menu is a plain chest menu any client shows.

**Install both:** `vastpacks_native-<version>.hnmod` (the native mod: build natives in the launcher after) and
`vastpacks-<version>.hmod` (the items, art, recipes and names).

## Not yet

Upgrades (pickup, magnet, feeding, restock), opening with a key, wearing it, and bigger tiers with pages (a chest
menu holds at most 54 slots).

## Building

`python3 tools/package.py` writes `dist/vastpacks_native-<v>.hnmod` and `dist/vastpacks-<v>.hmod`
(`tools/build_content.py` makes the content, with every sprite drawn in it; `tools/icon.py` the icon).

## License

MIT, see [LICENSE](LICENSE). The idea of keeping a backpack's contents in the world by id comes from Sophisticated
Backpacks; no code, art or text of it is used, and this mod is not affiliated with its authors.
