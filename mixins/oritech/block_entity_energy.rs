// The FE buffer of both machines (API 142, `Capabilities.Energy.BLOCK`).
fn observe_block_entity_energy(
    block: &str,
    energy: &mut Option<horizon_client::containers::ModEnergyBuffer>,
) {
    crate::game::oritech_state::on_block_entity_energy(block, energy);
}
