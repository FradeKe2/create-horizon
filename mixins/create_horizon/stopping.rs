// The server stops: forget the networks (Create's are per level, in memory).
fn create_stopping() {
    crate::game::create_mod::reset();
}
