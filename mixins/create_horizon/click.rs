// Right clicks on Create's blocks: turning the hand crank, setting the
// creative motor, and andesite alloy on a stripped log (item application).
fn create_click(game: &mut crate::game::Game<'_>, pos: [i32; 3], canceled: &mut bool) {
    use crate::game::create_mod as m;
    if *canceled || !game.native_on(m::ID) {
        return;
    }
    if m::apply_item(game, pos) {
        *canceled = true;
        return;
    }
    let name = game.native_block(pos);
    if name != "create:hand_crank" && name != "create:creative_motor" {
        return;
    }
    let sneaking = game
        .player
        .as_ref()
        .is_some_and(|p| p.pose == horizon_client::pose::Pose::Crouching);
    let said = m::right_click(game, pos, sneaking);
    *canceled = true;
    if let Some(line) = said {
        let who = game.native_acting_id();
        game.native_action_bar(who, horizon_client::text::Text::from(line));
    }
}
