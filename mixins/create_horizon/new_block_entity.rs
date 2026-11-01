// Each kinetic block gets its block entity (`EntityBlock.newBlockEntity`).
fn create_new_block_entity(block: &str, entity: &mut Option<(String, horizon_world::nbt::Compound)>) {
    crate::game::create_mod::new_block_entity(block, entity);
}
