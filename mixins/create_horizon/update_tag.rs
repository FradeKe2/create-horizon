// What clients get of a kinetic block entity (`getUpdateTag`): its speed,
// source and network's stress, for drawing.
fn create_update_tag(
    entity: &horizon_client::containers::ModBlockEntity,
    tag: &mut Option<horizon_world::nbt::Compound>,
) {
    if crate::game::create_mod::is_ours(&entity.id) {
        *tag = Some(crate::game::create_mod::update_tag(entity));
    }
}
