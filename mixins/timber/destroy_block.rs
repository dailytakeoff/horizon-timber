// Injected at the HEAD of Game::native_destroy_block: hands the break to Timber,
// then the game's own break of the block the player hit carries on as normal.
fn timber_destroy_block(game: &mut crate::game::Game<'_>, pos: [i32; 3], canceled: &mut bool) {
    crate::game::timber_glue::on_destroy_block(game, pos, canceled);
}
