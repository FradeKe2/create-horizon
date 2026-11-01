use crate::blocks::{Block, Kind};
use crate::geom::{Axis, Direction, Pos};
use crate::memory::MemoryWorld;
use crate::render;
use crate::state::{Kinetic, Read, Value};

fn shaft(axis: Axis) -> Block {
    Block::axial(Kind::Shaft, axis)
}
fn cog(axis: Axis) -> Block {
    Block::axial(Kind::Cogwheel, axis)
}
fn large(axis: Axis) -> Block {
    Block::axial(Kind::LargeCogwheel, axis)
}
fn motor(facing: Direction) -> Block {
    Block::facing(Kind::CreativeMotor, facing)
}

/// A motor at the origin facing east and shafts along +x.
fn motor_line(len: i32) -> MemoryWorld {
    let mut w = MemoryWorld::default();
    w.place([0, 0, 0], motor(Direction::East));
    for x in 1..=len {
        w.place([x, 0, 0], shaft(Axis::X));
    }
    w
}

#[test]
fn a_motor_turns_a_line_of_shafts() {
    let mut w = motor_line(5);
    w.ticks(3);
    assert_eq!(w.speed([0, 0, 0]), 16.0);
    for x in 1..=5 {
        assert_eq!(w.speed([x, 0, 0]), 16.0, "shaft {x}");
        assert_eq!(w.entities[&[x, 0, 0]].source, Some([x - 1, 0, 0]));
    }
    // One network, the motor's, holding all six.
    assert_eq!(w.nets.map.len(), 1);
    let net = w.nets.map.values().next().unwrap();
    assert_eq!(net.members.len(), 6);
    assert_eq!(net.current_capacity, 16384.0 * 16.0);
}

#[test]
fn a_motor_facing_west_turns_the_other_way() {
    let mut w = MemoryWorld::default();
    w.place([0, 0, 0], motor(Direction::West));
    w.place([-1, 0, 0], shaft(Axis::X));
    w.ticks(2);
    assert_eq!(w.speed([0, 0, 0]), -16.0);
    assert_eq!(w.speed([-1, 0, 0]), -16.0);
}

#[test]
fn a_shaft_off_the_motor_axis_stays_still() {
    let mut w = MemoryWorld::default();
    w.place([0, 0, 0], motor(Direction::East));
    w.place([1, 0, 0], shaft(Axis::Z));
    w.ticks(2);
    assert_eq!(w.speed([1, 0, 0]), 0.0);
}

#[test]
fn meshed_cogwheels_turn_opposite_ways() {
    let mut w = motor_line(1);
    w.place([2, 0, 0], cog(Axis::X));
    w.place([2, 1, 0], cog(Axis::X));
    w.place([2, 2, 0], cog(Axis::X));
    w.ticks(3);
    assert_eq!(w.speed([2, 0, 0]), 16.0);
    assert_eq!(w.speed([2, 1, 0]), -16.0);
    assert_eq!(w.speed([2, 2, 0]), 16.0);
}

#[test]
fn cogwheels_on_different_axes_do_not_mesh() {
    let mut w = motor_line(1);
    w.place([2, 0, 0], cog(Axis::X));
    w.place([2, 1, 0], cog(Axis::Z));
    w.ticks(3);
    assert_eq!(w.speed([2, 1, 0]), 0.0);
}

#[test]
fn a_large_cog_doubles_a_small_cog_beside_it() {
    // Large cog on the line's axis, a small cog diagonally beside it.
    let mut w = motor_line(1);
    w.place([2, 0, 0], large(Axis::X));
    w.place([2, 1, 1], cog(Axis::X));
    w.ticks(3);
    assert_eq!(w.speed([2, 0, 0]), 16.0);
    assert_eq!(w.speed([2, 1, 1]), -32.0);
}

#[test]
fn a_small_cog_halves_into_a_large_cog() {
    let mut w = motor_line(1);
    w.place([2, 0, 0], cog(Axis::X));
    w.place([2, 1, 1], large(Axis::X));
    w.ticks(3);
    assert_eq!(w.speed([2, 1, 1]), -8.0);
}

#[test]
fn two_large_cogs_at_right_angles_turn_each_other() {
    let mut w = motor_line(1);
    w.place([2, 0, 0], large(Axis::X));
    w.place([3, 1, 0], large(Axis::Y));
    w.ticks(3);
    assert_eq!(w.speed([3, 1, 0]).abs(), 16.0);
}

#[test]
fn a_gearbox_reverses_straight_through_and_turns_sideways() {
    let mut w = motor_line(1);
    w.place([2, 0, 0], Block::axial(Kind::Gearbox, Axis::Y));
    w.place([3, 0, 0], shaft(Axis::X));
    w.place([2, 0, 1], shaft(Axis::Z));
    w.place([2, 0, -1], shaft(Axis::Z));
    w.ticks(3);
    assert_eq!(w.speed([2, 0, 0]), 16.0);
    assert_eq!(w.speed([3, 0, 0]), -16.0);
    // Sideways: the halves facing a positive direction turn one way, the
    // negative ones the other (`getAxisModifier`).
    // From the west: south (positive) keeps the sign, north flips it.
    assert_eq!(w.speed([2, 0, 1]), 16.0);
    assert_eq!(w.speed([2, 0, -1]), -16.0);
}

#[test]
fn a_gearbox_does_not_pass_along_its_own_axis() {
    let mut w = MemoryWorld::default();
    w.place([0, 0, 0], motor(Direction::Up));
    w.place([0, 1, 0], Block::axial(Kind::Gearbox, Axis::Y));
    w.ticks(3);
    assert_eq!(w.speed([0, 1, 0]), 0.0);
}

#[test]
fn breaking_a_shaft_stops_what_it_drove() {
    let mut w = motor_line(5);
    w.ticks(3);
    w.break_block([3, 0, 0]);
    w.ticks(2);
    assert_eq!(w.speed([2, 0, 0]), 16.0);
    assert_eq!(w.speed([4, 0, 0]), 0.0);
    assert_eq!(w.speed([5, 0, 0]), 0.0);
    assert_eq!(w.entities[&[4, 0, 0]].network, None);
    // Putting it back turns them again.
    w.place([3, 0, 0], shaft(Axis::X));
    w.ticks(2);
    assert_eq!(w.speed([5, 0, 0]), 16.0);
}

#[test]
fn breaking_the_motor_stops_everything() {
    let mut w = motor_line(3);
    w.ticks(3);
    w.break_block([0, 0, 0]);
    w.ticks(2);
    for x in 1..=3 {
        assert_eq!(w.speed([x, 0, 0]), 0.0);
    }
    assert!(w.nets.map.is_empty());
}

#[test]
fn opposing_motors_break_a_block() {
    let mut w = MemoryWorld::default();
    w.place([0, 0, 0], motor(Direction::East));
    w.place([1, 0, 0], shaft(Axis::X));
    // Faces west, into the shaft: placed on a running line it follows it
    // (`initialize` only takes over when faster)...
    w.place([2, 0, 0], motor(Direction::West));
    w.ticks(3);
    assert!(w.destroyed.is_empty());
    assert_eq!(w.speed([2, 0, 0]), 16.0);
    // ...and set to turn against it, something breaks (`applyNewSpeed`).
    w.with(|e| e.set_motor_speed([2, 0, 0], 16));
    w.ticks(1);
    assert!(!w.destroyed.is_empty(), "something should have broken");
}

#[test]
fn a_faster_motor_takes_over_a_network() {
    let mut w = MemoryWorld::default();
    w.place([0, 0, 0], motor(Direction::East));
    w.place([1, 0, 0], shaft(Axis::X));
    w.place([2, 0, 0], shaft(Axis::X));
    w.ticks(3);
    // A second motor, same direction along x, faster.
    w.place([3, 0, 0], motor(Direction::West));
    w.with(|e| e.set_motor_speed([3, 0, 0], -64));
    w.ticks(3);
    assert!(w.destroyed.is_empty(), "{:?}", w.destroyed);
    assert_eq!(w.speed([1, 0, 0]), 64.0);
    assert_eq!(w.speed([0, 0, 0]), 64.0);
}

#[test]
fn changing_the_motor_speed_reaches_the_line() {
    let mut w = motor_line(3);
    w.ticks(2);
    w.with(|e| e.set_motor_speed([0, 0, 0], 128));
    w.ticks(2);
    assert_eq!(w.speed([3, 0, 0]), 128.0);
    w.with(|e| e.set_motor_speed([0, 0, 0], 0));
    w.ticks(2);
    assert_eq!(w.speed([3, 0, 0]), 0.0);
}

#[test]
fn too_fast_breaks() {
    // 256 rpm into a large cog: the small cog would turn at 512.
    let mut w = MemoryWorld::default();
    w.place([0, 0, 0], motor(Direction::East));
    w.with(|e| e.set_motor_speed([0, 0, 0], 256));
    w.place([1, 0, 0], large(Axis::X));
    w.place([1, 1, 1], cog(Axis::X));
    w.ticks(3);
    assert!(!w.destroyed.is_empty());
}

#[test]
fn a_hand_crank_turns_for_ten_ticks() {
    let mut w = MemoryWorld::default();
    // The crank faces up; its shaft points down.
    w.place([0, 1, 0], Block::facing(Kind::HandCrank, Direction::Up));
    w.place([0, 0, 0], shaft(Axis::Y));
    w.ticks(1);
    w.with(|e| e.crank([0, 1, 0], false));
    w.ticks(1);
    assert_eq!(w.speed([0, 1, 0]), 32.0);
    assert_eq!(w.speed([0, 0, 0]), 32.0);
    w.ticks(10);
    assert_eq!(w.speed([0, 1, 0]), 0.0);
    assert_eq!(w.speed([0, 0, 0]), 0.0);
    // Sneaking turns it backwards.
    w.with(|e| e.crank([0, 1, 0], true));
    w.ticks(1);
    assert_eq!(w.speed([0, 0, 0]), -32.0);
}

#[test]
fn a_hand_crank_overstresses_two_presses() {
    let mut w = MemoryWorld::default();
    w.place([0, 0, 0], Block::facing(Kind::HandCrank, Direction::West));
    w.place([1, 0, 0], Block::axial(Kind::MechanicalPress, Axis::X));
    w.ticks(1);
    w.with(|e| e.crank([0, 0, 0], false));
    w.ticks(2);
    // 8 su/rpm of capacity for 8 su/rpm of impact: just enough.
    assert!(!w.entities[&[1, 0, 0]].over_stressed);
    assert_ne!(w.speed([1, 0, 0]), 0.0);

    w.place([2, 0, 0], Block::axial(Kind::MechanicalPress, Axis::X));
    w.with(|e| e.crank([0, 0, 0], false));
    w.ticks(2);
    let k = &w.entities[&[1, 0, 0]];
    assert!(k.over_stressed, "{k:?}");
    assert_eq!(w.speed([1, 0, 0]), 0.0);
    assert_eq!(w.speed([2, 0, 0]), 0.0);
    assert_eq!(k.stress, 2.0 * 8.0 * 32.0);
    assert_eq!(k.capacity, 8.0 * 32.0);
}

#[test]
fn a_creative_motor_carries_a_heavy_load() {
    let mut w = motor_line(1);
    for x in 2..12 {
        w.place([x, 0, 0], Block::axial(Kind::MechanicalPress, Axis::X));
    }
    w.ticks(4);
    assert!(!w.entities[&[5, 0, 0]].over_stressed);
    assert_eq!(w.speed([11, 0, 0]), 16.0);
    assert_eq!(w.entities[&[11, 0, 0]].stress, 10.0 * 8.0 * 16.0);
}

#[test]
fn the_stressometer_reads_the_network() {
    let mut w = motor_line(1);
    w.place([2, 0, 0], Block::axial(Kind::Stressometer, Axis::X));
    w.place([3, 0, 0], Block::axial(Kind::MechanicalPress, Axis::X));
    w.ticks(4);
    let k = &w.entities[&[2, 0, 0]];
    let dial = render::stress_dial(k.stress, k.capacity, k.over_stressed);
    assert!((dial - (8.0 / 16384.0)).abs() < 1e-6, "{dial}");
}

#[test]
fn saved_fields_round_trip() {
    let mut w = motor_line(2);
    w.ticks(3);
    let k = w.entities[&[2, 0, 0]].clone();
    let saved = k.write(false);
    let back = Kinetic::read(&Fields(&saved));
    assert_eq!(back.speed, k.speed);
    assert_eq!(back.source, k.source);
    assert_eq!(back.network, k.network);
    assert_eq!(back.capacity, k.capacity);
    assert_eq!(back.stress, k.stress);
    assert_eq!(back.network_size, k.network_size);
    assert!(!back.update_speed);
    // A fresh block entity still updates.
    assert!(Kinetic::read(&Fields(&[])).update_speed);
}

#[test]
fn a_reloaded_network_keeps_turning() {
    let mut w = motor_line(3);
    w.ticks(3);
    // Save and load every block entity, and forget the networks, as a world
    // reopened does.
    let saved: Vec<(Pos, Vec<(&'static str, Value)>)> =
        w.entities.iter().map(|(p, k)| (*p, k.write(false))).collect();
    for (p, s) in saved {
        w.entities.insert(p, Kinetic::read(&Fields(&s)));
    }
    w.nets.map.clear();
    w.ticks(3);
    for x in 0..=3 {
        assert_eq!(w.speed([x, 0, 0]), 16.0);
    }
    let net = w.nets.map.values().next().unwrap();
    assert_eq!(net.members.len(), 4);
    assert_eq!(net.current_capacity, 16384.0 * 16.0);
}

#[test]
fn render_offsets_follow_create() {
    let b = cog(Axis::Y);
    assert!(render::should_offset(Axis::Y, [0, 5, 0]));
    assert!(!render::should_offset(Axis::Y, [1, 5, 0]));
    assert_eq!(render::rotation_offset(&large(Axis::Y), Axis::Y, [1, 0, 0]), 11.25);
    // 16 rpm: 4.8 degrees a tick, a full turn in 75 ticks.
    assert!((render::angle(&b, [1, 0, 0], 16.0, 75.0) - 0.0).abs() < 1e-3);
    assert!((crate::engine::convert_to_angular(16.0) - 4.8).abs() < 1e-6);
    assert_eq!(render::speed_dial(30.0), 0.45);
}

/// Saved fields as the glue reads them back from NBT.
struct Fields<'a>(&'a [(&'static str, Value)]);

impl Read for Fields<'_> {
    fn get(&self, key: &str) -> Option<Value> {
        self.0.iter().find(|(k, _)| *k == key).map(|(_, v)| v.clone())
    }
    fn sub(&self, key: &str) -> Option<Box<dyn Read + '_>> {
        match self.0.iter().find(|(k, _)| *k == key) {
            Some((_, Value::Compound(c))) => Some(Box::new(Fields(c))),
            _ => None,
        }
    }
}

