// Vast Packs, the game's side: injected into horizon-client's game.rs as `vastpacks_client`.
//
// A backpack item carries only an id (`minecraft:custom_data` `vastpacks_id`); what it holds lives in the world, in
// `<world>/vastpacks/<id>.dat`, as Sophisticated Backpacks keeps it in the world's saved data. So a backpack thrown,
// dropped or lost while it is open takes nothing with it, and nothing can be counted twice:
// - a right click (main hand) gives the backpack its id and asks for its menu, opened on the next server tick;
// - the menu is a chest menu whose slots are all input slots, filled from the file;
// - when it closes, for any reason, its slots are written to the file and only then taken; a backpack put in one
//   stays in its slot and goes back to the player, and if the file can't be written everything goes back.

use horizon_client::items::ItemStack;
use horizon_world::nbt::{JavaString, Tag};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

/// The backpacks: their rows of 9 slots, and their pages. Past a chest menu's 54 slots (Vast Depths' metals) a page
/// holds 45 and the last row turns them.
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
const ID_KEY: &str = "vastpacks_id";

/// A backpack's menu: its rows, the slots of a page, and its pages.
fn layout(item: &str) -> Option<(u8, usize, usize)> {
    let (_, rows, pages) = TIERS.iter().find(|(id, _, _)| *id == item)?;
    Some(if *pages > 1 { (6, PER_PAGE, *pages) } else { (*rows, usize::from(*rows) * 9, 1) })
}

fn is_backpack(item: &str) -> bool {
    layout(item).is_some()
}

/// A stack that is a backpack or carries one anywhere in its components (a bundle, a shulker box, a bundle in a
/// shulker box...): it never goes into a backpack, or a backpack could end up locked inside itself.
fn holds_backpack(stack: &ItemStack) -> bool {
    fn walk(t: &Tag) -> bool {
        match t {
            Tag::String(s) => is_backpack(&s.to_string_lossy()),
            Tag::List(l) => l.iter().any(walk),
            Tag::Compound(c) => c.iter().any(|(_, v)| walk(v)),
            _ => false,
        }
    }
    is_backpack(&stack.item) || horizon_items::item_nbt::item_to_nbt(stack).get("components").is_some_and(walk)
}

/// A backpack waiting for its menu: the player, its id, its item, and the ticks it may still wait (a menu that can't
/// open this tick, the player not in reach of this level's tick, is tried again).
struct Pending {
    uuid: u128,
    id: String,
    item: String,
    page: usize,
    /// A page turned: it waits for the last page to be written instead of being refused as already open.
    turning: bool,
    tries: u8,
}

/// An open backpack: its id, its item, the page shown, the slots a page has and its pages.
struct Open {
    id: String,
    item: String,
    page: usize,
    per_page: usize,
    pages: usize,
}

static PENDING: Mutex<Vec<Pending>> = Mutex::new(Vec::new());
static OPEN: Mutex<Option<HashMap<(u128, i32), Open>>> = Mutex::new(None);
static COUNTER: AtomicU64 = AtomicU64::new(0);

fn is_open(id: &str) -> bool {
    OPEN.lock().is_ok_and(|o| o.as_ref().is_some_and(|m| m.values().any(|v| v.id == id)))
}

/// The id a backpack carries.
fn id_of(stack: &ItemStack) -> Option<String> {
    match stack.data()?.other("minecraft:custom_data")?.get(ID_KEY)? {
        Tag::String(s) => Some(s.to_string_lossy()),
        _ => None,
    }
}

/// Gives a backpack the id `id`, keeping its other custom data.
fn set_id(stack: &mut ItemStack, id: &str) {
    let mut data = stack.data().and_then(|d| d.other("minecraft:custom_data")).cloned().unwrap_or_else(|| Tag::compound(Vec::<(&str, Tag)>::new()));
    if let Tag::Compound(map) = &mut data {
        map.insert(JavaString::from(ID_KEY), Tag::String(id.into()));
    }
    stack.data_mut().set_other("minecraft:custom_data", Some(data));
}

/// A new id: the time and a counter, in hex (a file name).
fn new_id() -> String {
    let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
    format!("{:x}{:04x}", nanos, COUNTER.fetch_add(1, Ordering::Relaxed) & 0xffff)
}

fn file(game: &super::Game<'_>, id: &str) -> Option<PathBuf> {
    // An id is only ever hex digits; anything else is refused rather than used as a path.
    if id.is_empty() || !id.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    Some(game.native_world_dir()?.join("vastpacks").join(format!("{id}.dat")))
}

/// What the backpack `id` holds, in `size` slots (more than it was saved with when it was upgraded).
fn load(game: &super::Game<'_>, id: &str, size: usize) -> Vec<Option<ItemStack>> {
    let read = file(game, id)
        .and_then(|p| std::fs::read(p).ok())
        .and_then(|bytes| horizon_world::nbt::read_named(&bytes, horizon_world::nbt::Limits::default()).ok());
    match read {
        Some((named, _)) => horizon_items::item_nbt::decode_items(named.tag.get("Items"), size),
        None => vec![None; size],
    }
}

/// Writes what the backpack `id` holds; false when it couldn't be.
fn save(game: &super::Game<'_>, id: &str, items: &[Option<ItemStack>]) -> bool {
    let Some(path) = file(game, id) else { return false };
    let tag = Tag::compound(vec![("Items", horizon_items::item_nbt::encode_items(items))]);
    let named = horizon_world::nbt::NamedTag { name: JavaString::from(""), tag };
    let Ok(bytes) = horizon_world::nbt::write_named(&named, horizon_world::nbt::Limits::default()) else { return false };
    let Some(dir) = path.parent() else { return false };
    // A new file, then renamed over the old one: a crash halfway leaves the old contents whole.
    let tmp = path.with_extension("tmp");
    std::fs::create_dir_all(dir).is_ok() && std::fs::write(&tmp, bytes).is_ok() && std::fs::rename(&tmp, &path).is_ok()
}

fn say(game: &mut super::Game<'_>, key: &str, fallback: &str) {
    let text = horizon_client::lang::get_or(key, fallback);
    game.native_feedback(horizon_client::text::Text::legacy(&format!("§7{text}")));
}

/// At `native_use_item`: a backpack in the main hand opens (on the next server tick).
pub(super) fn use_item(game: &mut super::Game<'_>, stack: &ItemStack, claimed: &mut bool) {
    if *claimed || game.remote.is_some() || !is_backpack(&stack.item) {
        return;
    }
    if game.hand != horizon_client::first_person::Hand::Main {
        return;
    }
    *claimed = true;
    let Some(player) = game.native_acting_player() else { return };
    if game.native_world_dir().is_none() {
        say(game, "vastpacks.no_world", "Backpacks open in a world.");
        return;
    }
    let slot = game.selected;
    let Some(held) = game.inventory[slot].as_mut().filter(|s| s.item == stack.item) else { return };
    let id = match id_of(held) {
        Some(id) => id,
        None => {
            let id = new_id();
            set_id(held, &id);
            id
        }
    };
    if is_open(&id) {
        say(game, "vastpacks.busy", "That backpack is already open.");
        return;
    }
    if let Ok(mut p) = PENDING.lock()
        && !p.iter().any(|q| q.id == id)
    {
        p.push(Pending { uuid: player.uuid, id, item: stack.item.to_string(), page: 0, turning: false, tries: 40 });
    }
}

/// The last row of a backpack with pages: the previous page, which page this is (as many papers), the next.
fn nav(page: usize, pages: usize) -> Vec<Option<ItemStack>> {
    // Names as translatable text, so each player reads them in their own language.
    let named = |item: &str, count: u32, name: Tag| {
        let mut s = ItemStack::new(item, count);
        s.data_mut().set_other("minecraft:custom_name", Some(name));
        Some(s)
    };
    let tr = |key: &str, with: Vec<String>| {
        let mut parts = vec![("translate", Tag::String(key.into()))];
        if !with.is_empty() {
            parts.push(("with", Tag::List(with.iter().map(|w| Tag::String(w.as_str().into())).collect())));
        }
        Tag::compound(parts)
    };
    let filler = || named("minecraft:gray_stained_glass_pane", 1, Tag::String(" ".into()));
    let mut row: Vec<Option<ItemStack>> = (0..9).map(|_| filler()).collect();
    if page > 0 {
        row[0] = named("minecraft:arrow", 1, tr("vastpacks.previous", Vec::new()));
    }
    row[4] = named("minecraft:paper", (page + 1) as u32, tr("vastpacks.page", vec![(page + 1).to_string(), pages.to_string()]));
    if page + 1 < pages {
        row[8] = named("minecraft:arrow", 1, tr("vastpacks.next", Vec::new()));
    }
    row
}

/// At the head of `native_server_tick`: the menus asked for.
pub(super) fn server_tick(game: &mut super::Game<'_>) {
    if game.remote.is_some() {
        return;
    }
    let pending: Vec<Pending> = PENDING.lock().map(|mut p| std::mem::take(&mut *p)).unwrap_or_default();
    let mut again = Vec::new();
    for mut p in pending {
        let Some((rows, per_page, pages)) = layout(&p.item) else { continue };
        let page = p.page.min(pages - 1);
        // A backpack opens once at a time; a page turned waits for the last page's slots to be written.
        if is_open(&p.id) {
            if p.turning && p.tries > 0 {
                p.tries -= 1;
                again.push(p);
            }
            continue;
        }
        let all = load(game, &p.id, per_page * pages);
        let mut items: Vec<Option<ItemStack>> = all[page * per_page..(page + 1) * per_page].to_vec();
        if pages > 1 {
            items.extend(nav(page, pages));
        }
        // Translatable, so each player reads it in their own language.
        use horizon_client::text::Text;
        let name = Text::translate(&format!("item.{}", p.item.replace(':', ".")), &[]);
        let title = if pages > 1 {
            Text::translate("vastpacks.title_paged", &[name, Text::literal((page + 1).to_string()), Text::literal(pages.to_string())])
        } else {
            name
        };
        match game.native_open_menu_with_input(p.uuid, rows, title, items, 0..per_page) {
            Some(window) => {
                if let Ok(mut o) = OPEN.lock() {
                    o.get_or_insert_with(HashMap::new)
                        .insert((p.uuid, window), Open { id: p.id, item: p.item, page, per_page, pages });
                }
            }
            None if p.tries > 0 => {
                p.tries -= 1;
                again.push(p);
            }
            None => log::warn!("Vast Packs: backpack {} could not be opened for its player", p.id),
        }
    }
    if !again.is_empty()
        && let Ok(mut q) = PENDING.lock()
    {
        q.extend(again);
    }
}

/// At `native_menu_clicked`: the last row of a backpack with pages turns them. The menu closes first (its page is
/// written), then the next one opens.
pub(super) fn menu_clicked(game: &mut super::Game<'_>, uuid: u128, window: i32, slot: i32) {
    let turn = OPEN.lock().ok().and_then(|o| {
        let open = o.as_ref()?.get(&(uuid, window))?;
        if open.pages < 2 {
            return None;
        }
        let at = usize::try_from(slot).ok()?.checked_sub(open.per_page)?;
        let page = match at {
            0 if open.page > 0 => open.page - 1,
            8 if open.page + 1 < open.pages => open.page + 1,
            _ => return None,
        };
        Some(Pending { uuid, id: open.id.clone(), item: open.item.clone(), page, turning: true, tries: 40 })
    });
    if let Some(turn) = turn {
        game.native_close_menu(uuid);
        if let Ok(mut p) = PENDING.lock() {
            p.push(turn);
        }
    }
}

/// At `native_server_about_to_start`: a new server keeps nothing of the last one's menus (a close never heard
/// would leave a backpack "already open" for good).
pub(super) fn server_about_to_start() {
    if let Ok(mut o) = OPEN.lock() {
        *o = None;
    }
    if let Ok(mut p) = PENDING.lock() {
        p.clear();
    }
}

/// At `native_menu_input_closed`: a backpack's menu closed; its slots go to its file.
pub(super) fn menu_input_closed(game: &mut super::Game<'_>, uuid: u128, window: i32, slots: &mut Vec<Option<ItemStack>>) {
    let open = OPEN.lock().ok().and_then(|mut o| o.as_mut()?.remove(&(uuid, window)));
    let Some(open) = open else { return };
    // What it keeps: everything but backpacks and what carries one, which go back to the player.
    let size = open.per_page;
    let mut keep: Vec<Option<ItemStack>> = vec![None; size];
    for (i, slot) in slots.iter().enumerate().take(size) {
        if let Some(s) = slot
            && !holds_backpack(s)
        {
            keep[i] = Some(s.clone());
        }
    }
    // This page's slots over the rest of the backpack.
    let mut all = load(game, &open.id, open.per_page * open.pages);
    for (i, k) in keep.iter().enumerate() {
        all[open.page * open.per_page + i] = k.clone();
    }
    if save(game, &open.id, &all) {
        for (i, slot) in slots.iter_mut().enumerate().take(size) {
            if keep[i].is_some() {
                *slot = None;
            }
        }
    } else {
        // Nothing taken: everything goes back to the player.
        log::warn!("Vast Packs: backpack {} could not be saved; its items go back to the player", open.id);
    }
}
