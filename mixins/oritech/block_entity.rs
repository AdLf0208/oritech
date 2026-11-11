// Both machines get a block entity (API 15/129).
fn observe_block_entity(block: &str, entity: &mut Option<(String, horizon_world::nbt::Compound)>) {
    crate::game::oritech_state::on_block_entity(block, entity);
}
