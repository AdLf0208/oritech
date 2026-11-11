// `/oritech info|energy` (API 8).
fn observe_register_commands(commands: &mut Vec<NativeCommand>) {
    crate::game::oritech_state::on_register_commands(commands);
}
