// A right click inserts fuel / a recipe input, or reads the machine's status.
fn observe_right_click_block(
    game: &mut crate::game::Game<'_>,
    pos: [i32; 3],
    face: [i32; 3],
    canceled: &mut bool,
) {
    crate::game::oritech_state::on_right_click_block(game, pos, face, canceled);
}
