//! Vast Packs: backpacks. A right click (main hand) opens one as a chest menu whose slots are the backpack's own:
//! what it holds lives in the item (its container component), so it travels with it, and the game keeps the
//! backpack from going into itself and closes the menu when it leaves the hand. Past a chest menu's 54 slots (Vast
//! Depths' metals) a page holds 45 and the last row turns them.

mod migrate;

use mod_api::{
    self, Hand, HostSide, Hooks, InteractionResult, ItemPart, ItemStack, MenuClickedEvent, MenuClosedEvent,
    MenuReplace, MenuSlotPlace, Registrar, RegistrationContext, ServerAboutToStartEvent, ServerHost, Text,
    UseItemEvent,
};
use std::collections::HashMap;
use std::sync::Mutex;

/// The backpacks: their rows of 9 slots, and their pages.
const TIERS: [(&str, u8, usize); 8] = [
    ("vastpacks:leather_backpack", 2, 1),
    ("vastpacks:iron_backpack", 3, 1),
    ("vastpacks:gold_backpack", 4, 1),
    ("vastpacks:diamond_backpack", 5, 1),
    ("vastpacks:netherite_backpack", 6, 1),
    ("vastpacks:stellarite_backpack", 6, 2),
    ("vastpacks:infernium_backpack", 6, 3),
    ("vastpacks:voidium_backpack", 6, 4),
];
const PER_PAGE: usize = 45;

/// A backpack's menu: its rows, the slots a page shows, and its pages.
fn layout(item: &str) -> Option<(u8, usize, usize)> {
    let (_, rows, pages) = TIERS.iter().find(|(id, _, _)| *id == item)?;
    Some(if *pages > 1 { (6, PER_PAGE, *pages) } else { (*rows, usize::from(*rows) * 9, 1) })
}

/// The slots a backpack holds in all.
fn capacity(item: &str) -> Option<usize> {
    layout(item).map(|(_, per_page, pages)| per_page * pages)
}

/// An open backpack menu: the backpack, the page shown, the slots a page has and its pages.
struct Open {
    item: &'static str,
    page: usize,
    per_page: usize,
    pages: usize,
}

static OPEN: Mutex<Option<HashMap<(u128, i32), Open>>> = Mutex::new(None);

fn tr(key: &str, args: Vec<Text>) -> Text {
    Text::translate(key, args)
}

/// A stack named with a translatable text, for the page row.
fn named(host: &dyn mod_api::Host, item: &str, count: u32, name: mod_api::Compound) -> Option<ItemStack> {
    let key = host.item_key(&mod_api::Identifier::parse(item).ok()?)?;
    let mut stack = ItemStack::new(key, count)?;
    let mut patch = mod_api::Compound::new();
    patch.insert(java("minecraft:custom_name"), mod_api::Tag::Compound(name));
    stack.apply_components(&patch).ok()?;
    Some(stack)
}

fn java<K: for<'a> From<&'a str>>(s: &str) -> K {
    K::from(s)
}

/// A text component as NBT: `{"translate": key, "with": [...], "italic": false}`.
fn component(key: &str, with: &[String]) -> mod_api::Compound {
    use mod_api::Tag;
    let mut c = mod_api::Compound::new();
    c.insert(java("translate"), Tag::String(java(key)));
    if !with.is_empty() {
        c.insert(java("with"), Tag::List(with.iter().map(|w| Tag::String(java(w))).collect()));
    }
    c.insert(java("italic"), Tag::Byte(0));
    c
}

/// The last row of a backpack with pages: the previous page, which page this is (as many papers), the next.
fn nav(host: &dyn mod_api::Host, page: usize, pages: usize) -> Vec<Option<ItemStack>> {
    let mut blank = mod_api::Compound::new();
    blank.insert(java("text"), mod_api::Tag::String(java(" ")));
    let mut row: Vec<Option<ItemStack>> = (0..9).map(|_| named(host, "minecraft:gray_stained_glass_pane", 1, blank.clone())).collect();
    if page > 0 {
        row[0] = named(host, "minecraft:arrow", 1, component("vastpacks.previous", &[]));
    }
    row[4] = named(host, "minecraft:paper", (page + 1) as u32, component("vastpacks.page", &[(page + 1).to_string(), pages.to_string()]));
    if page + 1 < pages {
        row[8] = named(host, "minecraft:arrow", 1, component("vastpacks.next", &[]));
    }
    row
}

/// Opens page `page` of the backpack `item` held in the player's main hand, its slots backed by the backpack's own.
fn open(host: &mut dyn ServerHost, uuid: u128, item: &'static str, page: usize, replace: MenuReplace) {
    let Some((rows, per_page, pages)) = layout(item) else { return };
    let page = page.min(pages - 1);
    let mut items: Vec<Option<ItemStack>> = vec![None; per_page];
    if pages > 1 {
        items.extend(nav(host, page, pages));
    }
    let name = tr(&format!("item.{}", item.replace(':', ".")), Vec::new());
    let title = if pages > 1 {
        tr("vastpacks.title_paged", vec![name, Text::literal((page + 1).to_string()), Text::literal(pages.to_string())])
    } else {
        name
    };
    let Ok(Some(window)) = host.open_menu(uuid, rows, title, items, replace) else { return };
    let Some(player) = host.player(uuid) else { return };
    let places = (0..per_page).map(|i| MenuSlotPlace::HeldItem { hand: Hand::Main, slot: page * per_page + i }).collect();
    match player.back_mod_menu_slots(host, window, places) {
        Ok(()) => {
            if let Ok(mut o) = OPEN.lock() {
                o.get_or_insert_with(HashMap::new).insert((uuid, window), Open { item, page, per_page, pages });
            }
        }
        Err(refusal) => {
            eprintln!("Vast Packs: backpack menu not backed: {refusal:?}");
            host.close_menu(uuid);
        }
    }
}

/// A 0.1 backpack (its contents in a file of the world) gets them back in itself, once: the held stack is replaced by
/// one holding them, and the file is renamed so it is never read twice.
fn move_old_contents(host: &mut dyn ServerHost, uuid: u128, held: &ItemStack, item: &str) {
    let Some(id) = held.components().and_then(|c| migrate::old_id(c.as_nbt())) else { return };
    let Some(world) = host.world_dir() else { return };
    let path = migrate::file(&world, &id);
    let Ok(bytes) = std::fs::read(&path) else { return };
    let Some(patch) = capacity(item).and_then(|slots| migrate::patch_from(&bytes, slots)) else {
        eprintln!("Vast Packs: {} could not be read", path.display());
        return;
    };
    let mut stack = held.clone();
    if stack.apply_components(&patch).is_err() {
        eprintln!("Vast Packs: {} holds items this world can't make", path.display());
        return;
    }
    let Some(living) = host.player(uuid).and_then(|p| host.living(p.entity())) else { return };
    if matches!(living.set_hand_item(host, Hand::Main, Some(stack)), Ok(true)) {
        let _ = std::fs::rename(&path, path.with_extension("dat.moved"));
    }
}

/// A backpack's item: used in the main hand, it opens.
struct Backpack {
    item: &'static str,
}

#[mod_api::horizon_part]
impl ItemPart for Backpack {
    /// The server opens the backpack's menu; the client lets the use through.
    fn use_item(&self, host: HostSide, event: &mut UseItemEvent<'_>) -> Option<InteractionResult> {
        if event.hand != Hand::Main {
            return None;
        }
        match host {
            HostSide::Server(host) => {
                move_old_contents(host, event.player, event.stack, self.item);
                open(host, event.player, self.item, 0, MenuReplace::CloseFirst);
                Some(InteractionResult::SuccessServer)
            }
            HostSide::Client(_) => Some(InteractionResult::Success),
        }
    }
}

#[derive(Default)]
struct VastPacks;

#[mod_api::horizon_mod]
impl Hooks for VastPacks {
    fn register(&self, r: &mut dyn Registrar, _: &RegistrationContext) {
        for (id, _, _) in TIERS {
            let key = r.item_key(id);
            let slots = capacity(id).unwrap_or(0) as u32;
            r.item_container(&key, move |_: &dyn mod_api::Host| Some(slots));
            r.item_part(key, Backpack { item: id });
        }
    }

    /// The last row of a backpack with pages turns them: the next page opens in the menu's place.
    fn menu_clicked(&self, host: &mut dyn ServerHost, event: &mut MenuClickedEvent) {
        let turn = OPEN.lock().ok().and_then(|o| {
            let open = o.as_ref()?.get(&(event.uuid, event.window))?;
            if open.pages < 2 {
                return None;
            }
            let at = usize::try_from(event.slot).ok()?.checked_sub(open.per_page)?;
            let page = match at {
                0 if open.page > 0 => open.page - 1,
                8 if open.page + 1 < open.pages => open.page + 1,
                _ => return None,
            };
            Some((open.item, page))
        });
        if let Some((item, page)) = turn {
            open(host, event.uuid, item, page, MenuReplace::InPlace);
        }
    }

    fn menu_closed(&self, _host: &mut dyn ServerHost, event: &mut MenuClosedEvent) {
        if let Ok(mut o) = OPEN.lock()
            && let Some(map) = o.as_mut()
        {
            map.remove(&(event.uuid, event.window));
        }
    }

    /// A new server keeps nothing of the last one's menus.
    fn server_about_to_start(&self, _host: &mut dyn ServerHost, _event: &mut ServerAboutToStartEvent) {
        if let Ok(mut o) = OPEN.lock() {
            *o = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layouts() {
        assert_eq!(layout("vastpacks:leather_backpack"), Some((2, 18, 1)));
        assert_eq!(layout("vastpacks:netherite_backpack"), Some((6, 54, 1)));
        assert_eq!(layout("vastpacks:voidium_backpack"), Some((6, 45, 4)));
        assert_eq!(capacity("vastpacks:infernium_backpack"), Some(135));
        assert_eq!(layout("minecraft:chest"), None);
    }
}
