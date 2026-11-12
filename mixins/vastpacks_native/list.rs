// The mod's row in Options > Mods.
fn vastpacks_listed(mods: &mut Vec<crate::game::native_hooks::NativeMod>) {
    mods.push(crate::game::native_hooks::NativeMod {
        id: "vastpacks_native".into(),
        title: "Vast Packs 0.1.0".into(),
        description: "Backpacks: right-click to open; what they hold is kept in the world.".into(),
        keys: Vec::new(),
    });
}
