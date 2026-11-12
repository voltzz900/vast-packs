// A new server: no menu of the last one is still open.
fn vastpacks_server_about_to_start(game: &mut crate::game::Game<'_>, dedicated: bool) {
    let _ = (game, dedicated);
    crate::game::vastpacks_client::server_about_to_start();
}
