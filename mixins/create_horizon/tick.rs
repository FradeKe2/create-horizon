// A server tick of a kinetic block entity (`KineticBlockEntity.tick`).
fn create_tick(
    game: &mut crate::game::Game<'_>,
    pos: [i32; 3],
    entity: &mut horizon_client::containers::ModBlockEntity,
) {
    if !game.native_on(crate::game::create_mod::ID) {
        return;
    }
    crate::game::create_mod::tick(game, pos, entity);
}
