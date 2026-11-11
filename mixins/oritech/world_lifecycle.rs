// A world opened or closed: the recipe cache and the push queue are per world.
fn observe_world_lifecycle(game: &mut crate::game::Game<'_>, phase: WorldPhase) {
    crate::game::oritech_state::on_world_lifecycle(game, phase);
}
