# Registry page: Vast Packs

- **File:** `vastpacks-0.2.0.hmod` (release v0.2.0), kind: native mod
- **Icon:** `icons/vastpacks.png`
- **Name:** Vast Packs
- **Summary:** Backpacks: right-click to open like a chest, from 18 to 54 slots (and up to 180 in pages with Vast Depths); what they hold travels with them.
- **Categories:** storage, equipment, utility
- **Tags:** backpacks, storage, inventory, translation
- **License:** MIT
- **Source:** https://github.com/voltzz900/vast-packs
- **Authors:** voltzz900

## Description

Backpacks on Horizon's mod API: hold one in your main hand, right-click, and it opens like a chest. One package: the code, the items, their sprites, recipes and names in English and Brazilian Portuguese.

| Backpack | Slots |
| --- | --- |
| Leather | 18 |
| Iron | 27 |
| Gold | 36 |
| Diamond | 45 |
| Netherite | 54 |
| Stellarite, Infernium, Voidium (need Vast Depths) | 90, 135, 180, in pages of 45 |

- **Upgrades** in the crafting grid (the backpack and a block of the next material) keep everything it holds.
- **What it holds is in the backpack itself** (its container, as a shulker box's): drop it, trade it, keep it in a chest, it travels with its contents.
- **Safe by design**: the menu's slots are the backpack's own storage, so every move goes through it in one transaction; the backpack in your hand can't go into itself, and the menu closes once it leaves your hand.

**What it changes in the game**: through the mod API (an item container on each backpack, a chest menu backed by the held backpack, the `use_item`, `menu_clicked` and `menu_closed` hooks; no mixins). Backpacks of 0.1.0 (which kept their contents in `<world>/vastpacks/`) get them back the first time they're opened; that file is then renamed, never deleted. No network of its own.

0.2.0 replaces 0.1.0's two files (`vastpacks_native` .hnmod and the `vastpacks` content mod). Pinned to Horizon 702ab1e.
