// Drawing a kinetic block entity (Create's `KineticBlockEntityRenderer`):
// its shafts and cogs turn at its speed.
fn create_submit(
    game: &crate::game::Game<'_>,
    pos: [i32; 3],
    block: &str,
    state: horizon_world::registry::BlockStateId,
    entity: &horizon_client::containers::ModBlockEntity,
    draw: &mut crate::game::native_block_entity_draw::BlockEntityDraw,
) {
    if crate::game::create_mod::is_ours(&entity.id) {
        crate::game::create_mod::submit(game, pos, block, state, entity, draw);
    }
}
