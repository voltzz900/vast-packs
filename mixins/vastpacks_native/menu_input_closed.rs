// A backpack's menu closed: what it holds goes to its file.
fn vastpacks_menu_input_closed(
    game: &mut crate::game::Game<'_>,
    uuid: u128,
    window: i32,
    slots: &mut Vec<Option<horizon_client::items::ItemStack>>,
) {
    crate::game::vastpacks_client::menu_input_closed(game, uuid, window, slots);
}
