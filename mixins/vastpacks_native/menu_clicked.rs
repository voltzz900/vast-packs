// A click in a backpack's last row turns its pages.
fn vastpacks_menu_clicked(game: &mut crate::game::Game<'_>, uuid: u128, window: i32, slot: i32) {
    crate::game::vastpacks_client::menu_clicked(game, uuid, window, slot);
}
