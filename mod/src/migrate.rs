//! Backpacks of 0.1: their contents were kept in the world, in `<world>/vastpacks/<id>.dat` (a named NBT compound,
//! uncompressed, its `Items` a list of `{Slot, id, count, components}`), the backpack carrying only
//! `minecraft:custom_data.vastpacks_id`. The file names no owner, so whoever holds a stack with that id could claim
//! it: the contents move into a backpack (its container component) only while an operator holds it. The same patch
//! drops the id, so a backpack is never imported twice, and the file is then renamed `<id>.dat.moved`; a backpack
//! that still has the id and is empty (the game stopped before it was saved) is filled again from that file.

use mod_api::{Compound, Tag};
use std::path::{Path, PathBuf};

fn java<K: for<'a> From<&'a str>>(s: &str) -> K {
    K::from(s)
}

fn get<'a>(nbt: &'a Compound, key: &str) -> Option<&'a Tag> {
    nbt.iter().find(|(k, _)| k.0.iter().copied().eq(key.encode_utf16())).map(|(_, v)| v)
}

fn text(tag: &Tag) -> Option<String> {
    match tag {
        Tag::String(s) => Some(String::from_utf16_lossy(&s.0)),
        _ => None,
    }
}

/// The 0.1 id a backpack's components carry, if any.
pub fn old_id(components: &Compound) -> Option<String> {
    let Some(Tag::Compound(data)) = get(components, "minecraft:custom_data") else { return None };
    let id = text(get(data, "vastpacks_id")?)?;
    // An id was only ever hex digits; anything else is not a file name of ours.
    (!id.is_empty() && id.chars().all(|c| c.is_ascii_hexdigit())).then_some(id)
}

/// The 0.1 file of backpack `id` in `world`.
pub fn file(world: &Path, id: &str) -> PathBuf {
    world.join("vastpacks").join(format!("{id}.dat"))
}

/// The name a 0.1 file has once its contents were moved.
pub fn moved(file: &Path) -> PathBuf {
    file.with_extension("dat.moved")
}

/// Whether the stack's container component holds anything.
pub fn holds_items(components: &Compound) -> bool {
    matches!(get(components, "minecraft:container"), Some(Tag::List(entries)) if !entries.is_empty())
}

/// Where to read a backpack's 0.1 contents from: its file if it is still there, or the moved file when the backpack
/// is empty (an import whose backpack was never saved); `None` otherwise. The bool says whether it is the
/// unmoved file.
pub fn source(file: &Path, empty: bool) -> Option<(PathBuf, bool)> {
    if file.is_file() {
        Some((file.to_path_buf(), true))
    } else if empty && moved(file).is_file() {
        Some((moved(file), false))
    } else {
        None
    }
}

/// The backpack's custom data without the 0.1 id, as a patch entry.
fn custom_data_without_id(components: &Compound) -> Compound {
    match get(components, "minecraft:custom_data") {
        Some(Tag::Compound(data)) => {
            data.iter().filter(|(k, _)| !k.0.iter().copied().eq("vastpacks_id".encode_utf16())).map(|(k, v)| (k.clone(), v.clone())).collect()
        }
        _ => Compound::new(),
    }
}

/// The component patch that puts a 0.1 file's items into the backpack (`minecraft:container`, by slot), for the
/// slots it has, and takes the 0.1 id out of its custom data; `None` when the file can't be read.
pub fn patch_from(bytes: &[u8], slots: usize, components: &Compound) -> Option<Compound> {
    let root = read_named(bytes)?;
    let mut entries = Vec::new();
    if let Some(Tag::List(items)) = get(&root, "Items") {
        for item in items {
            let Tag::Compound(item) = item else { continue };
            let slot = match get(item, "Slot") {
                Some(Tag::Byte(b)) => i32::from(*b as u8),
                Some(Tag::Int(i)) => *i,
                Some(Tag::Short(s)) => i32::from(*s),
                _ => continue,
            };
            if slot < 0 || slot as usize >= slots {
                continue;
            }
            let stack: Compound = item.iter().filter(|(k, _)| !k.0.iter().copied().eq("Slot".encode_utf16())).map(|(k, v)| (k.clone(), v.clone())).collect();
            let mut entry = Compound::new();
            entry.insert(java("slot"), Tag::Int(slot));
            entry.insert(java("item"), Tag::Compound(stack));
            entries.push(Tag::Compound(entry));
        }
    }
    let mut patch = Compound::new();
    patch.insert(java("minecraft:container"), Tag::List(entries));
    patch.insert(java("minecraft:custom_data"), Tag::Compound(custom_data_without_id(components)));
    Some(patch)
}

// ---- a small reader for Java's NBT (big-endian, modified UTF-8) ----------------------------------------------------

struct Reader<'a> {
    b: &'a [u8],
    i: usize,
}

impl Reader<'_> {
    fn take(&mut self, n: usize) -> Option<&[u8]> {
        let s = self.b.get(self.i..self.i.checked_add(n)?)?;
        self.i += n;
        Some(s)
    }
    fn u8(&mut self) -> Option<u8> {
        self.take(1).map(|s| s[0])
    }
    fn i16(&mut self) -> Option<i16> {
        self.take(2).map(|s| i16::from_be_bytes([s[0], s[1]]))
    }
    fn i32(&mut self) -> Option<i32> {
        self.take(4).map(|s| i32::from_be_bytes(s.try_into().unwrap()))
    }
    fn i64(&mut self) -> Option<i64> {
        self.take(8).map(|s| i64::from_be_bytes(s.try_into().unwrap()))
    }
    fn len(&mut self) -> Option<usize> {
        usize::try_from(self.i32()?).ok().filter(|n| *n <= self.b.len())
    }
    fn string(&mut self) -> Option<String> {
        let n = self.i16()? as u16 as usize;
        let raw = self.take(n)?;
        Some(modified_utf8(raw))
    }
    fn payload(&mut self, kind: u8, depth: u32) -> Option<Tag> {
        if depth > 512 {
            return None;
        }
        Some(match kind {
            1 => Tag::Byte(self.u8()? as i8),
            2 => Tag::Short(self.i16()?),
            3 => Tag::Int(self.i32()?),
            4 => Tag::Long(self.i64()?),
            5 => Tag::Float(self.i32()? as u32),
            6 => Tag::Double(self.i64()? as u64),
            7 => {
                let n = self.len()?;
                Tag::ByteArray(self.take(n)?.to_vec())
            }
            8 => Tag::String(java(&self.string()?)),
            9 => {
                let inner = self.u8()?;
                let n = self.len()?;
                let mut list = Vec::with_capacity(n.min(4096));
                for _ in 0..n {
                    list.push(self.payload(inner, depth + 1)?);
                }
                Tag::List(list)
            }
            10 => Tag::Compound(self.compound(depth + 1)?),
            11 => {
                let n = self.len()?;
                Tag::IntArray((0..n).map(|_| self.i32()).collect::<Option<_>>()?)
            }
            12 => {
                let n = self.len()?;
                Tag::LongArray((0..n).map(|_| self.i64()).collect::<Option<_>>()?)
            }
            _ => return None,
        })
    }
    fn compound(&mut self, depth: u32) -> Option<Compound> {
        let mut c = Compound::new();
        loop {
            let kind = self.u8()?;
            if kind == 0 {
                return Some(c);
            }
            let name = self.string()?;
            let value = self.payload(kind, depth)?;
            c.insert(java(&name), value);
        }
    }
}

/// Java's modified UTF-8: NUL as two bytes, characters past U+FFFF as surrogate pairs of three bytes each.
fn modified_utf8(raw: &[u8]) -> String {
    let mut units: Vec<u16> = Vec::with_capacity(raw.len());
    let mut i = 0;
    while i < raw.len() {
        let a = u16::from(raw[i]);
        if a < 0x80 {
            units.push(a);
            i += 1;
        } else if a & 0xE0 == 0xC0 && i + 1 < raw.len() {
            units.push(((a & 0x1F) << 6) | (u16::from(raw[i + 1]) & 0x3F));
            i += 2;
        } else if i + 2 < raw.len() {
            units.push(((a & 0x0F) << 12) | ((u16::from(raw[i + 1]) & 0x3F) << 6) | (u16::from(raw[i + 2]) & 0x3F));
            i += 3;
        } else {
            break;
        }
    }
    String::from_utf16_lossy(&units)
}

/// A named root compound.
fn read_named(bytes: &[u8]) -> Option<Compound> {
    let mut r = Reader { b: bytes, i: 0 };
    if r.u8()? != 10 {
        return None;
    }
    let _name = r.string()?;
    r.compound(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A 0.1 file: `{"": {Items: [{Slot: 3b, id: "minecraft:stone", count: 5}]}}`.
    fn sample() -> Vec<u8> {
        let mut b = vec![10, 0, 0];
        b.extend([9, 0, 5]);
        b.extend(b"Items");
        b.extend([10, 0, 0, 0, 1]);
        b.extend([1, 0, 4]);
        b.extend(b"Slot");
        b.push(3);
        b.extend([8, 0, 2]);
        b.extend(b"id");
        b.extend([0, 15]);
        b.extend(b"minecraft:stone");
        b.extend([3, 0, 5]);
        b.extend(b"count");
        b.extend([0, 0, 0, 5]);
        b.push(0); // end of the item
        b.push(0); // end of the root
        b
    }

    #[test]
    fn moves_items_by_slot() {
        let patch = patch_from(&sample(), 18, &Compound::new()).unwrap();
        let Some(Tag::List(entries)) = get(&patch, "minecraft:container") else { panic!("no container") };
        assert_eq!(entries.len(), 1);
        let Tag::Compound(e) = &entries[0] else { panic!() };
        assert_eq!(get(e, "slot"), Some(&Tag::Int(3)));
        let Some(Tag::Compound(item)) = get(e, "item") else { panic!() };
        assert_eq!(get(item, "count"), Some(&Tag::Int(5)));
        assert!(get(item, "Slot").is_none());
        // A slot past the backpack's is left out.
        let patch = patch_from(&sample(), 2, &Compound::new()).unwrap();
        assert_eq!(get(&patch, "minecraft:container"), Some(&Tag::List(Vec::new())));
    }

    #[test]
    fn the_patch_drops_the_old_id_and_keeps_other_data() {
        let mut data = Compound::new();
        data.insert(java("vastpacks_id"), Tag::String(java("19a0f3")));
        data.insert(java("other"), Tag::Int(7));
        let mut c = Compound::new();
        c.insert(java("minecraft:custom_data"), Tag::Compound(data));
        let patch = patch_from(&sample(), 18, &c).unwrap();
        assert_eq!(old_id(&patch), None);
        let Some(Tag::Compound(kept)) = get(&patch, "minecraft:custom_data") else { panic!("no custom data") };
        assert_eq!(get(kept, "other"), Some(&Tag::Int(7)));
        // Applied, the patch leaves a backpack the migration no longer picks up.
        assert!(holds_items(&patch));
    }

    #[test]
    fn the_source_is_the_file_or_for_an_empty_backpack_the_moved_one() {
        let dir = std::env::temp_dir().join(format!("vastpacks-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let world = dir.as_path();
        std::fs::create_dir_all(world.join("vastpacks")).unwrap();
        let f = file(world, "ab");
        assert_eq!(source(&f, true), None);
        std::fs::write(&f, sample()).unwrap();
        assert_eq!(source(&f, false), Some((f.clone(), true)));
        std::fs::rename(&f, moved(&f)).unwrap();
        // Moved: only a backpack that is still empty reads it again.
        assert_eq!(source(&f, false), None);
        assert_eq!(source(&f, true), Some((moved(&f), false)));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn old_ids_are_hex_only() {
        let mut data = Compound::new();
        data.insert(java("vastpacks_id"), Tag::String(java("19a0f3")));
        let mut c = Compound::new();
        c.insert(java("minecraft:custom_data"), Tag::Compound(data.clone()));
        assert_eq!(old_id(&c).as_deref(), Some("19a0f3"));
        data.insert(java("vastpacks_id"), Tag::String(java("../x")));
        c.insert(java("minecraft:custom_data"), Tag::Compound(data));
        assert_eq!(old_id(&c), None);
    }
}

#[cfg(test)]
mod real_file {
    /// Reads a real 0.1 file named by VASTPACKS_SAMPLE (skipped when unset).
    #[test]
    fn reads_a_real_file() {
        let Ok(path) = std::env::var("VASTPACKS_SAMPLE") else { return };
        let bytes = std::fs::read(path).unwrap();
        let patch = super::patch_from(&bytes, 180, &mod_api::Compound::new()).expect("readable");
        let Some(mod_api::Tag::List(entries)) = super::get(&patch, "minecraft:container") else { panic!("no container") };
        for e in entries {
            let mod_api::Tag::Compound(e) = e else { continue };
            let Some(mod_api::Tag::Compound(item)) = super::get(e, "item") else { continue };
            let id = super::get(item, "id").and_then(super::text).unwrap_or_default();
            eprintln!("slot {:?} {id} {:?}", super::get(e, "slot"), super::get(item, "count"));
        }
    }
}
