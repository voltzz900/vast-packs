# Registry page: Vast Packs

- **File:** `vastbackpacks-0.3.1.hmod` (release v0.3.1), kind: native mod, id `vastbackpacks`
- **Icon:** `icons/vastbackpacks.png`
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

**What it changes in the game**: through the mod API (an item container on each backpack, a chest menu backed by the held backpack, the `use_item`, `menu_clicked`, `menu_closed` and `mod_mismatch` hooks; no mixins). Backpacks of 0.1.0 (which kept their contents in `<world>/vastpacks/`) get them back once, the first time one is opened while an operator holds it; that file is then renamed, never deleted. No network of its own.

The project's id is `vastbackpacks`: `vastpacks` stays the registry's content-only 0.1.0 (the half whose native part was never published). The items keep their `vastpacks:` ids, so backpacks saved with 0.1 and 0.2 stay in their worlds, and a world that recorded the old `vastpacks` code mod opens without the missing-mods question (the mod resolves that id when no `vastpacks` mod is loaded). Turn the old `vastpacks` content mod off: two mods can't own one namespace. Pinned to Horizon 0b14f8a.

Known issue: on the pinned engine the backpack menu closes as soon as it opens in single player, and can on a dedicated server too, because Horizon can't yet back a menu's slots during the player's own turn (horizon-rust/horizon#18513; fix: horizon-rust/horizon#19593). In creative a shift-click can fill a second backpack instead of the held one (horizon-rust/horizon#18515), fixed in the game by horizon-rust/horizon#19595 after this pin.
