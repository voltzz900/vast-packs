# Registry page: Vast Packs (native)

- **File:** `vastpacks_native-0.1.0.hnmod` (release v0.1.0), kind: native mod
- **Icon:** `icons/vastpacks.png`
- **Name:** Vast Packs
- **Summary:** Backpacks: right-click to open like a chest, five tiers from 18 to 54 slots, upgrades keep what they hold. Needs its content mod.
- **Categories:** storage, equipment, utility
- **Tags:** backpacks, storage, inventory
- **License:** MIT
- **Source:** https://github.com/voltzz900/vast-packs
- **Authors:** voltzz900

## Description

Backpacks for Horizon. **Install the content mod `vastpacks` too** (the items, their art, recipes and names in English and Brazilian Portuguese).

- **Five tiers:** Leather (18 slots), Iron (27), Gold (36), Diamond (45) and Netherite (54). Right-click one in your main hand and it opens like a chest.
- **Upgrades** in the crafting grid (the backpack and a block of iron, gold or diamond, or a netherite ingot) keep everything it holds.
- **Safe:** the item carries only an id and its contents live in the world (`<world>/vastpacks/<id>.dat`, carried by backups), so a backpack dropped or lost while open takes nothing with it and nothing is counted twice; a backpack put in a backpack goes back to you; contents are written when the menu closes for any reason, atomically, and go back to you if they can't be.
- Works on servers: the menu is a plain chest menu.

**What it changes in the game:** a right click with a backpack in the main hand (`native_use_item`, claimed) gives the item an id in its custom data and opens, on the next server tick, a chest menu whose slots are input slots (`native_open_menu_with_input`); when that menu closes (`native_menu_input_closed`) its slots are written to `<world>/vastpacks/<id>.dat` and only then taken. It reads and writes only that folder. No network of its own.

Not yet: pickup and magnet upgrades, a key to open, wearing it. The idea of keeping contents in the world by id comes from Sophisticated Backpacks; no code, art or text of it is used.
