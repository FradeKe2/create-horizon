// The axis or facing a kinetic block is placed with.
fn create_placement(
    level: &crate::game::GameLevel<'_>,
    state: horizon_world::registry::BlockStateId,
    context: &horizon_world::level::PlacementContext,
    answer: &mut Option<Option<horizon_world::registry::BlockStateId>>,
) {
    use crate::game::create_mod as m;
    if answer.is_some() {
        return;
    }
    let Some(name) = level.registry.state(state).map(|s| s.name.clone()) else { return };
    if !m::is_ours(&name) {
        return;
    }
    let face = create_kinetics::Direction::ALL
        .into_iter()
        .find(|d| d.normal() == context.face.normal())
        .unwrap_or(create_kinetics::Direction::Up);
    let Some((key, value)) = m::placement(&name, face, context.yaw, context.pitch, context.sneaking) else {
        return;
    };
    if let Some(placed) = level.native_state_with_property(state, key, value) {
        *answer = Some(Some(placed));
    }
}
