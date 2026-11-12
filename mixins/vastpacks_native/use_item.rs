// A backpack in the main hand opens.
fn vastpacks_use_item(game: &mut crate::game::Game<'_>, stack: &horizon_client::items::ItemStack, claimed: &mut bool) {
    crate::game::vastpacks_client::use_item(game, stack, claimed);
}
