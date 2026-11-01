// Every kinetic block entity ticks (`getTicker`).
fn create_ticks(block: &str, ticks: &mut bool) {
    if crate::game::create_mod::is_ours(block) {
        *ticks = true;
    }
}
