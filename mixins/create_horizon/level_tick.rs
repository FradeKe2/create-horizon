// Before each level tick: detach the kinetic blocks removed since.
fn create_level_tick(game: &mut crate::game::Game<'_>, dimension: &str) {
    crate::game::create_mod::level_tick(game, dimension);
}
