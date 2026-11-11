// The generator's fuel slot, the pulverizer's input and two outputs (API 16).
fn observe_block_entity_container(
    block: &str,
    container: &mut Option<horizon_client::containers::ModContainer>,
) {
    crate::game::oritech_state::on_container(block, container);
}
