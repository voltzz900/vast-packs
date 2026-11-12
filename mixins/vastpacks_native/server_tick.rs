// The backpack menus asked for.
fn vastpacks_server_tick(game: &mut crate::game::Game<'_>) {
    crate::game::vastpacks_client::server_tick(game);
}
