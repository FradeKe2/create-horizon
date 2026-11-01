// Create's row in Options > Mods.
fn create_listed(mods: &mut Vec<crate::game::native_hooks::NativeMod>) {
    use crate::game::create_mod as m;
    mods.push(crate::game::native_hooks::NativeMod {
        id: m::ID.into(),
        title: format!("{} {}", m::NAME, m::VERSION),
        description: "Create's kinetics: shafts, cogwheels, gearboxes, motors, cranks, gauges and stress.".into(),
        keys: Vec::new(),
    });
}
