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

/// The backpacks and their rows (9 slots each).
const TIERS: [(&str, u8); 5] = [
    ("vastpacks:leather_backpack", 2),
    ("vastpacks:iron_backpack", 3),
    ("vastpacks:gold_backpack", 4),
    ("vastpacks:diamond_backpack", 5),
    ("vastpacks:netherite_backpack", 6),
];
const ID_KEY: &str = "vastpacks_id";

fn rows(item: &str) -> Option<u8> {
    TIERS.iter().find(|(id, _)| *id == item).map(|(_, r)| *r)
}

fn is_backpack(item: &str) -> bool {
    rows(item).is_some()
}

/// A backpack waiting for its menu: the player, its id, its item.
struct Pending {
    uuid: u128,
    id: String,
    item: String,
}

/// An open backpack: its id and how many slots its menu has.
struct Open {
    id: String,
    size: usize,
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
        p.push(Pending { uuid: player.uuid, id, item: stack.item.to_string() });
    }
}

/// At the head of `native_server_tick`: the menus asked for.
pub(super) fn server_tick(game: &mut super::Game<'_>) {
    if game.remote.is_some() {
        return;
    }
    let pending: Vec<Pending> = PENDING.lock().map(|mut p| std::mem::take(&mut *p)).unwrap_or_default();
    for p in pending {
        let Some(rows) = rows(&p.item) else { continue };
        if is_open(&p.id) {
            continue;
        }
        let size = usize::from(rows) * 9;
        let items = load(game, &p.id, size);
        let name = p.item.trim_start_matches("vastpacks:");
        let title = horizon_client::text::Text::translate(&format!("item.vastpacks.{name}"), &[]);
        if let Some(window) = game.native_open_menu_with_input(p.uuid, rows, title, items, 0..size)
            && let Ok(mut o) = OPEN.lock()
        {
            o.get_or_insert_with(HashMap::new).insert((p.uuid, window), Open { id: p.id, size });
        }
    }
}

/// At `native_menu_input_closed`: a backpack's menu closed; its slots go to its file.
pub(super) fn menu_input_closed(game: &mut super::Game<'_>, uuid: u128, window: i32, slots: &mut Vec<Option<ItemStack>>) {
    let open = OPEN.lock().ok().and_then(|mut o| o.as_mut()?.remove(&(uuid, window)));
    let Some(open) = open else { return };
    // What it keeps: everything but backpacks, which go back to the player (a backpack in itself, or in another).
    let mut keep: Vec<Option<ItemStack>> = vec![None; open.size];
    for (i, slot) in slots.iter().enumerate().take(open.size) {
        if let Some(s) = slot
            && !is_backpack(&s.item)
        {
            keep[i] = Some(s.clone());
        }
    }
    if save(game, &open.id, &keep) {
        for (i, slot) in slots.iter_mut().enumerate().take(open.size) {
            if keep[i].is_some() {
                *slot = None;
            }
        }
    } else {
        // Nothing taken: everything goes back to the player.
        log::warn!("Vast Packs: backpack {} could not be saved; its items go back to the player", open.id);
    }
}
