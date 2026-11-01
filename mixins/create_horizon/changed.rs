// A kinetic block left its place: the next tick detaches it
// (`KineticBlockEntity.remove`).
fn create_changed(
    level: &crate::game::GameLevel<'_>,
    dimension: horizon_world::level::Dimension,
    pos: horizon_core::position::BlockPos,
    old: horizon_world::registry::BlockStateId,
    new: horizon_world::registry::BlockStateId,
) {
    use crate::game::create_mod as m;
    let block = |id: horizon_world::registry::BlockStateId| {
        let s = level.registry.state(id)?;
        m::block_of(&s.name, |k| s.properties.get(k).cloned())
    };
    let Some(before) = block(old) else { return };
    m::block_changed(m::dimension_id(dimension), [pos.x, pos.y, pos.z], Some(before), block(new));
}
