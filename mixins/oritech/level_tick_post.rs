// After the level's tick: move generator energy into adjacent pulverizers (API 23).
fn observe_level_tick_post(game: &mut crate::game::Game<'_>, dimension: &str) {
    crate::game::oritech_state::on_level_tick_post(game, dimension);
}
