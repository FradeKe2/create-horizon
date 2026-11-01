// Create's kinetic logic: `RotationPropagator`, `KineticNetwork`,
// `TorquePropagator` and the ticking of `KineticBlockEntity` and its
// generators, ported from Create (MIT, The Create Team / The Creators of Create)
// to Rust. Java works on block entity objects; here each step loads a block
// entity's fields from the world, changes them and stores them back, so the
// order of reads and writes follows the Java code's.

use crate::blocks::{Block, Kind};
use crate::geom::{Axis, Direction, Pos, add, as_long, manhattan, sub};
use crate::state::{Kinetic, MOTOR_MAX_SPEED};
use std::collections::{BTreeMap, HashMap, VecDeque};

/// `RotationPropagator.MAX_FLICKER_SCORE`.
pub const MAX_FLICKER_SCORE: i32 = 128;
/// `CKinetics.maxRotationSpeed` default.
pub const MAX_ROTATION_SPEED: f32 = 256.0;
/// `CKinetics.kineticValidationFrequency` default.
pub const VALIDATION_FREQUENCY: i32 = 60;
/// `HandCrankBlock.getRotationSpeed`.
pub const CRANK_SPEED: i32 = 32;

/// What the kinetics need of a level.
pub trait World {
    /// The kinetic block at `pos`, if there is one.
    fn block(&self, pos: Pos) -> Option<Block>;
    /// `Level.isLoaded`.
    fn loaded(&self, pos: Pos) -> bool;
    /// The kinetic block entity's fields at `pos`.
    fn kinetic(&self, pos: Pos) -> Option<Kinetic>;
    /// Stores them (`setChanged`).
    fn set_kinetic(&mut self, pos: Pos, k: Kinetic);
    /// `sendData`: clients get the block entity again.
    fn send_data(&mut self, pos: Pos);
    /// `Level.destroyBlock(pos, true)`: the block breaks and drops. Afterwards
    /// `block` and `kinetic` give nothing at `pos`.
    fn destroy_block(&mut self, pos: Pos);
}

/// `KineticNetwork`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Network {
    pub initialized: bool,
    /// Sources and their capacity per rpm.
    pub sources: BTreeMap<Pos, f32>,
    /// Members and their stress impact per rpm.
    pub members: BTreeMap<Pos, f32>,
    pub current_capacity: f32,
    pub current_stress: f32,
    pub unloaded_capacity: f32,
    pub unloaded_stress: f32,
    pub unloaded_members: i32,
}

impl Network {
    pub fn size(&self) -> i32 {
        self.unloaded_members + self.members.len() as i32
    }
}

/// `TorquePropagator`: one level's networks, by id.
#[derive(Clone, Debug, Default)]
pub struct Networks {
    pub map: HashMap<i64, Network>,
}

/// Java's `Math.signum` (zero for zero; Rust's `f32::signum` gives 1).
fn signum(x: f32) -> f32 {
    if x > 0.0 {
        1.0
    } else if x < 0.0 {
        -1.0
    } else {
        0.0
    }
}

/// `KineticBlockEntity.convertToDirection`.
pub fn convert_to_direction(axis_speed: f32, d: Direction) -> f32 {
    if d.positive() { axis_speed } else { -axis_speed }
}

/// `KineticBlockEntity.convertToAngular`: rpm to degrees per tick.
pub fn convert_to_angular(speed: f32) -> f32 {
    speed * 360.0 / 60.0 / 20.0
}

/// `getGeneratedSpeed` of the generators.
pub fn generated_speed(block: &Block, k: &Kinetic) -> f32 {
    match block.kind {
        Kind::CreativeMotor => match block.facing {
            Some(f) => convert_to_direction(k.motor_speed as f32, f),
            None => 0.0,
        },
        Kind::HandCrank => {
            let speed = if k.crank_in_use == 0 {
                0
            } else if k.crank_backwards {
                -1
            } else {
                1
            } * CRANK_SPEED;
            match block.facing {
                Some(f) => convert_to_direction(speed as f32, f),
                None => 0.0,
            }
        }
        _ => 0.0,
    }
}

pub struct Engine<'a, W: World> {
    pub w: &'a mut W,
    pub nets: &'a mut Networks,
}

impl<'a, W: World> Engine<'a, W> {
    pub fn new(w: &'a mut W, nets: &'a mut Networks) -> Self {
        Engine { w, nets }
    }

    // ---- loading and storing ----------------------------------------------

    fn kin(&self, pos: Pos) -> Option<Kinetic> {
        self.w.block(pos)?;
        self.w.kinetic(pos)
    }

    fn edit<R>(&mut self, pos: Pos, f: impl FnOnce(&mut Kinetic) -> R) -> Option<R> {
        let mut k = self.kin(pos)?;
        let r = f(&mut k);
        self.w.set_kinetic(pos, k);
        Some(r)
    }

    fn theoretical(&self, pos: Pos) -> f32 {
        self.kin(pos).map_or(0.0, |k| k.theoretical_speed())
    }

    fn speed(&self, pos: Pos) -> f32 {
        self.kin(pos).map_or(0.0, |k| k.speed())
    }

    fn generated(&self, pos: Pos) -> f32 {
        match (self.w.block(pos), self.w.kinetic(pos)) {
            (Some(b), Some(k)) => generated_speed(&b, &k),
            _ => 0.0,
        }
    }

    fn is_source(&self, pos: Pos) -> bool {
        self.generated(pos) != 0.0
    }

    fn set_speed(&mut self, pos: Pos, speed: f32) {
        self.edit(pos, |k| k.speed = speed);
    }

    // ---- KineticBlockEntity -----------------------------------------------

    /// `onSpeedChanged`.
    fn on_speed_changed(&mut self, pos: Pos, previous: f32) {
        self.edit(pos, |k| {
            let now = k.speed();
            let from_or_to_zero = (previous == 0.0) != (now == 0.0);
            let direction_swap = !from_or_to_zero && signum(previous) != signum(now);
            if from_or_to_zero || direction_swap {
                k.flicker_tally += 5;
            }
        });
    }

    /// `calculateStressApplied`.
    fn calculate_stress_applied(&mut self, pos: Pos) -> f32 {
        let impact = self.w.block(pos).map_or(0.0, |b| b.kind.stress_impact());
        self.edit(pos, |k| k.last_stress_applied = impact);
        impact
    }

    /// `calculateAddedStressCapacity`.
    fn calculate_added_capacity(&mut self, pos: Pos) -> f32 {
        let capacity = self.w.block(pos).map_or(0.0, |b| b.kind.stress_capacity());
        self.edit(pos, |k| k.last_capacity_provided = capacity);
        capacity
    }

    /// `updateFromNetwork`.
    fn update_from_network(&mut self, pos: Pos, max_stress: f32, current_stress: f32, size: i32) {
        let changed = self.edit(pos, |k| {
            k.network_dirty = false;
            k.capacity = max_stress;
            k.stress = current_stress;
            k.network_size = size;
            let over = max_stress < current_stress;
            if over != k.over_stressed {
                let prev = k.speed();
                k.over_stressed = over;
                Some(prev)
            } else {
                None
            }
        });
        if let Some(Some(prev)) = changed {
            self.on_speed_changed(pos, prev);
            self.w.send_data(pos);
        } else if changed.is_some() && self.w.block(pos).is_some_and(|b| b.kind == Kind::Stressometer) {
            // StressGaugeBlockEntity.updateFromNetwork sends its dial each time.
            self.w.send_data(pos);
        }
    }

    /// `setSource` (and `GeneratingKineticBlockEntity.setSource`).
    fn set_source(&mut self, pos: Pos, source: Pos) {
        self.edit(pos, |k| k.source = Some(source));
        let Some(src) = self.kin(source) else {
            self.remove_source(pos);
            return;
        };
        self.set_network(pos, src.network);
        if self.w.block(pos).is_some_and(|b| b.kind.is_generator()) {
            let generated = self.generated(pos);
            self.edit(pos, |k| {
                if k.reactivate_source && src.speed().abs() >= generated.abs() {
                    k.reactivate_source = false;
                }
            });
        }
    }

    /// `removeSource` (and the generators').
    fn remove_source(&mut self, pos: Pos) {
        if self.w.block(pos).is_some_and(|b| b.kind.is_generator()) {
            let is_source = self.is_source(pos);
            self.edit(pos, |k| {
                if k.has_source() && is_source {
                    k.reactivate_source = true;
                }
            });
        }
        let Some(prev) = self.edit(pos, |k| {
            let prev = k.speed();
            k.speed = 0.0;
            k.source = None;
            prev
        }) else {
            return;
        };
        self.set_network(pos, None);
        self.on_speed_changed(pos, prev);
    }

    /// `setNetwork`.
    fn set_network(&mut self, pos: Pos, network: Option<i64>) {
        let Some(k) = self.kin(pos) else { return };
        if k.network == network {
            return;
        }
        if let Some(old) = k.network {
            self.network_remove(old, pos);
        }
        self.edit(pos, |k| k.network = network);
        let Some(id) = network else { return };
        let net = self.nets.map.entry(id).or_default();
        net.initialized = true;
        self.network_add(id, pos);
    }

    /// `attachKinetics`.
    fn attach_kinetics(&mut self, pos: Pos) {
        self.edit(pos, |k| k.update_speed = false);
        self.handle_added(pos);
    }

    /// `detachKinetics`.
    fn detach_kinetics(&mut self, pos: Pos) {
        if let (Some(b), Some(k)) = (self.w.block(pos), self.w.kinetic(pos)) {
            self.handle_removed(pos, &b, &k);
        }
    }

    /// `validateKinetics`.
    fn validate_kinetics(&mut self, pos: Pos) {
        let Some(k) = self.kin(pos) else { return };
        if let Some(source) = k.source {
            if !k.has_network() {
                self.remove_source(pos);
                return;
            }
            if !self.w.loaded(source) {
                return;
            }
            let src = self.kin(source);
            if src.is_none_or(|s| s.speed == 0.0) {
                self.remove_source(pos);
                self.detach_kinetics(pos);
            }
            return;
        }
        if k.speed != 0.0 && self.generated(pos) == 0.0 {
            self.set_speed(pos, 0.0);
        }
    }

    /// `SmartBlockEntity.initialize` → `KineticBlockEntity.initialize` (and
    /// the creative motor's).
    fn initialize(&mut self, pos: Pos) {
        let Some(k) = self.kin(pos) else { return };
        if let Some(id) = k.network {
            let net = self.nets.map.entry(id).or_default();
            if !net.initialized {
                Self::init_from_te(net, k.capacity, k.stress, k.network_size);
            }
            self.add_silently(id, pos, k.last_capacity_provided, k.last_stress_applied);
        }
        if self.w.block(pos).is_some_and(|b| b.kind == Kind::CreativeMotor) {
            let k = self.kin(pos).unwrap_or_default();
            if !k.has_source() || self.generated(pos) > k.theoretical_speed() {
                self.update_generated_rotation(pos);
            }
        }
    }

    /// A server tick of the kinetic block entity at `pos`
    /// (`KineticBlockEntity.tick` and its subclasses').
    pub fn tick(&mut self, pos: Pos) {
        let Some(block) = self.w.block(pos) else { return };
        let Some(k) = self.w.kinetic(pos) else { return };
        if k.update_speed {
            self.attach_kinetics(pos);
        }
        // `super.tick()`: SmartBlockEntity initializes on its first tick.
        if self.kin(pos).is_some_and(|k| !k.initialized) {
            self.edit(pos, |k| k.initialized = true);
            self.initialize(pos);
        }
        let Some(countdown) = self.edit(pos, |k| {
            let c = k.validation_countdown;
            k.validation_countdown -= 1;
            c
        }) else {
            return;
        };
        if countdown <= 0 {
            self.edit(pos, |k| k.validation_countdown = VALIDATION_FREQUENCY);
            self.validate_kinetics(pos);
        }
        self.edit(pos, |k| {
            if k.flicker_tally > 0 {
                k.flicker_tally -= 1;
            }
        });
        if let Some(k) = self.kin(pos)
            && k.network_dirty
        {
            if let Some(id) = k.network {
                self.update_network(id);
            }
            self.edit(pos, |k| k.network_dirty = false);
        }

        if block.kind.is_generator() {
            // GeneratingKineticBlockEntity.tick
            if self.kin(pos).is_some_and(|k| k.reactivate_source) {
                self.update_generated_rotation(pos);
                self.edit(pos, |k| k.reactivate_source = false);
            }
        }
        if block.kind == Kind::HandCrank {
            // HandCrankBlockEntity.tick
            let stopped = self.edit(pos, |k| {
                if k.crank_in_use > 0 {
                    k.crank_in_use -= 1;
                    k.crank_in_use == 0
                } else {
                    false
                }
            });
            if stopped == Some(true) {
                self.update_generated_rotation(pos);
            }
        }
    }

    /// The block entity at `pos` goes (`KineticBlockEntity.remove`), with
    /// its block and fields as they were: call it once the block is gone.
    pub fn removed(&mut self, pos: Pos, block: &Block, k: &Kinetic) {
        if let Some(id) = k.network {
            self.network_remove_gone(id, pos, block, k);
        }
        self.handle_removed(pos, block, k);
    }

    /// `Level.destroyBlock` from the kinetics: the block entity is removed
    /// (`remove()`), then the block breaks.
    fn destroy(&mut self, pos: Pos) {
        let (Some(b), Some(k)) = (self.w.block(pos), self.w.kinetic(pos)) else {
            return;
        };
        if let Some(id) = k.network {
            self.network_remove(id, pos);
        }
        let k = self.w.kinetic(pos).unwrap_or(k);
        self.handle_removed(pos, &b, &k);
        self.w.destroy_block(pos);
    }

    // ---- GeneratingKineticBlockEntity -------------------------------------

    /// `updateGeneratedRotation`.
    pub fn update_generated_rotation(&mut self, pos: Pos) {
        let Some(k) = self.kin(pos) else { return };
        let speed = self.generated(pos);
        let prev = k.speed;
        if prev != speed {
            self.apply_new_speed(pos, prev, speed);
        }
        if let Some(k) = self.kin(pos)
            && let Some(id) = k.network
            && speed != 0.0
        {
            let cap = self.calculate_added_capacity(pos);
            self.update_capacity_for(id, pos, cap);
            let stress = self.calculate_stress_applied(pos);
            self.update_stress_for(id, pos, stress);
            self.update_stress(id);
        }
        self.on_speed_changed(pos, prev);
        self.w.send_data(pos);
    }

    /// `applyNewSpeed`.
    fn apply_new_speed(&mut self, pos: Pos, prev: f32, speed: f32) {
        let Some(k) = self.kin(pos) else { return };
        if speed == 0.0 {
            if k.has_source() {
                if let Some(id) = k.network {
                    self.update_capacity_for(id, pos, 0.0);
                    let stress = self.calculate_stress_applied(pos);
                    self.update_stress_for(id, pos, stress);
                }
                return;
            }
            self.detach_kinetics(pos);
            self.set_speed(pos, 0.0);
            self.set_network(pos, None);
            return;
        }
        if prev == 0.0 {
            self.set_speed(pos, speed);
            self.set_network(pos, Some(as_long(pos)));
            self.attach_kinetics(pos);
            return;
        }
        if k.has_source() {
            if prev.abs() >= speed.abs() {
                if signum(prev) != signum(speed) {
                    self.destroy(pos);
                }
                return;
            }
            self.detach_kinetics(pos);
            self.set_speed(pos, speed);
            self.edit(pos, |k| k.source = None);
            self.set_network(pos, Some(as_long(pos)));
            self.attach_kinetics(pos);
            return;
        }
        self.detach_kinetics(pos);
        self.set_speed(pos, speed);
        self.attach_kinetics(pos);
    }

    // ---- player actions ---------------------------------------------------

    /// `HandCrankBlockEntity.turn`.
    pub fn crank(&mut self, pos: Pos, back: bool) {
        let generated = self.generated(pos);
        let Some(update) = self.edit(pos, |k| {
            let update = generated == 0.0 || back != k.crank_backwards;
            k.crank_in_use = 10;
            k.crank_backwards = back;
            update
        }) else {
            return;
        };
        if update {
            self.update_generated_rotation(pos);
        }
    }

    /// The creative motor's scroll value changed (its callback).
    pub fn set_motor_speed(&mut self, pos: Pos, rpm: i32) {
        let rpm = rpm.clamp(-MOTOR_MAX_SPEED, MOTOR_MAX_SPEED);
        if self.edit(pos, |k| k.motor_speed = rpm).is_some() {
            self.update_generated_rotation(pos);
        }
    }

    // ---- KineticNetwork ---------------------------------------------------

    fn init_from_te(net: &mut Network, max_stress: f32, current_stress: f32, members: i32) {
        net.unloaded_capacity = max_stress;
        net.unloaded_stress = current_stress;
        net.unloaded_members = members;
        net.initialized = true;
        // `updateStress` and `updateCapacity` with no members loaded yet: the
        // unloaded totals.
        net.current_stress = net.unloaded_stress;
        net.current_capacity = net.unloaded_capacity;
    }

    fn add_silently(&mut self, id: i64, pos: Pos, last_capacity: f32, last_stress: f32) {
        if self.nets.map.get(&id).is_some_and(|n| n.members.contains_key(&pos)) {
            return;
        }
        let is_source = self.is_source(pos);
        let generated = self.generated(pos);
        let theoretical = self.theoretical(pos);
        let capacity = if is_source { Some(self.calculate_added_capacity(pos)) } else { None };
        let stress = self.calculate_stress_applied(pos);
        let net = self.nets.map.entry(id).or_default();
        if let Some(c) = capacity {
            net.unloaded_capacity -= last_capacity * generated.abs();
            net.sources.insert(pos, c);
        }
        net.unloaded_stress -= last_stress * theoretical.abs();
        net.members.insert(pos, stress);
        net.unloaded_members = (net.unloaded_members - 1).max(0);
        net.unloaded_capacity = net.unloaded_capacity.max(0.0);
        net.unloaded_stress = net.unloaded_stress.max(0.0);
    }

    fn network_add(&mut self, id: i64, pos: Pos) {
        if self.nets.map.get(&id).is_some_and(|n| n.members.contains_key(&pos)) {
            return;
        }
        if self.is_source(pos) {
            let c = self.calculate_added_capacity(pos);
            self.nets.map.entry(id).or_default().sources.insert(pos, c);
        }
        let s = self.calculate_stress_applied(pos);
        let net = self.nets.map.entry(id).or_default();
        net.members.insert(pos, s);
        let (cap, stress, size) = (net.current_capacity, net.current_stress, net.size());
        self.update_from_network(pos, cap, stress, size);
        self.edit(pos, |k| k.network_dirty = true);
    }

    fn network_remove(&mut self, id: i64, pos: Pos) {
        let Some(net) = self.nets.map.get(&id) else { return };
        if !net.members.contains_key(&pos) {
            return;
        }
        let is_source = self.is_source(pos);
        let net = self.nets.map.get_mut(&id).unwrap();
        if is_source {
            net.sources.remove(&pos);
        }
        net.members.remove(&pos);
        self.update_from_network(pos, 0.0, 0.0, 0);
        self.after_member_removed(id);
    }

    /// `remove` for a block entity no longer in the world.
    fn network_remove_gone(&mut self, id: i64, pos: Pos, block: &Block, k: &Kinetic) {
        let Some(net) = self.nets.map.get_mut(&id) else { return };
        if !net.members.contains_key(&pos) {
            return;
        }
        if generated_speed(block, k) != 0.0 {
            net.sources.remove(&pos);
        }
        net.members.remove(&pos);
        self.after_member_removed(id);
    }

    fn after_member_removed(&mut self, id: i64) {
        let Some(net) = self.nets.map.get(&id) else { return };
        if net.members.is_empty() {
            self.nets.map.remove(&id);
            return;
        }
        if let Some(first) = net.members.keys().next().copied() {
            self.edit(first, |k| k.network_dirty = true);
        }
    }

    fn update_capacity_for(&mut self, id: i64, pos: Pos, capacity: f32) {
        self.nets.map.entry(id).or_default().sources.insert(pos, capacity);
        self.update_capacity(id);
    }

    fn update_stress_for(&mut self, id: i64, pos: Pos, stress: f32) {
        self.nets.map.entry(id).or_default().members.insert(pos, stress);
        self.update_stress(id);
    }

    /// `sync`.
    fn sync(&mut self, id: i64) {
        let Some(net) = self.nets.map.get(&id) else { return };
        let members: Vec<Pos> = net.members.keys().copied().collect();
        for m in members {
            let Some(net) = self.nets.map.get(&id) else { return };
            let (cap, stress, size) = (net.current_capacity, net.current_stress, net.size());
            self.update_from_network(m, cap, stress, size);
        }
    }

    fn update_capacity(&mut self, id: i64) {
        let new = self.calculate_capacity(id);
        let Some(net) = self.nets.map.get_mut(&id) else { return };
        if net.current_capacity != new {
            net.current_capacity = new;
            self.sync(id);
        }
    }

    fn update_stress(&mut self, id: i64) {
        let new = self.calculate_stress(id);
        let Some(net) = self.nets.map.get_mut(&id) else { return };
        if net.current_stress != new {
            net.current_stress = new;
            self.sync(id);
        }
    }

    /// `updateNetwork`.
    pub fn update_network(&mut self, id: i64) {
        let stress = self.calculate_stress(id);
        let capacity = self.calculate_capacity(id);
        let Some(net) = self.nets.map.get_mut(&id) else { return };
        if net.current_stress != stress || net.current_capacity != capacity {
            net.current_stress = stress;
            net.current_capacity = capacity;
            self.sync(id);
        }
    }

    /// Whether the block entity at `pos` is still the one in network `id`
    /// (`level.getBlockEntity(pos) == be`).
    fn still_member(&self, id: i64, pos: Pos) -> bool {
        self.kin(pos).is_some_and(|k| k.network == Some(id))
    }

    fn calculate_capacity(&mut self, id: i64) -> f32 {
        let Some(net) = self.nets.map.get(&id) else { return 0.0 };
        let sources: Vec<(Pos, f32)> = net.sources.iter().map(|(p, c)| (*p, *c)).collect();
        let mut present = 0.0;
        let mut gone = Vec::new();
        for (p, c) in sources {
            if !self.still_member(id, p) {
                gone.push(p);
                continue;
            }
            present += c * self.generated(p).abs();
        }
        let net = self.nets.map.get_mut(&id).unwrap();
        for p in gone {
            net.sources.remove(&p);
        }
        present + net.unloaded_capacity
    }

    fn calculate_stress(&mut self, id: i64) -> f32 {
        let Some(net) = self.nets.map.get(&id) else { return 0.0 };
        let members: Vec<(Pos, f32)> = net.members.iter().map(|(p, s)| (*p, *s)).collect();
        let mut present = 0.0;
        let mut gone = Vec::new();
        for (p, s) in members {
            if !self.still_member(id, p) {
                gone.push(p);
                continue;
            }
            present += s * self.theoretical(p).abs();
        }
        let net = self.nets.map.get_mut(&id).unwrap();
        for p in gone {
            net.members.remove(&p);
        }
        present + net.unloaded_stress
    }

    // ---- RotationPropagator -----------------------------------------------

    /// `getRotationSpeedModifier`.
    pub fn rotation_speed_modifier(&self, from: Pos, to: Pos) -> f32 {
        let (Some(bf), Some(bt)) = (self.w.block(from), self.w.block(to)) else {
            return 0.0;
        };
        let diff = sub(to, from);
        let direction = Direction::nearest(diff);
        let aligned = Axis::ALL
            .into_iter()
            .filter(|a| *a != direction.axis())
            .all(|a| a.choose(diff) == 0);
        let by_axis = aligned
            && bf.has_shaft_towards(direction)
            && bt.has_shaft_towards(direction.opposite());
        let by_gears = bf.is_small_cog() && bt.is_small_cog();

        // Axis <-> Axis
        if by_axis {
            let mut m = self.axis_modifier(to, &bt, direction.opposite());
            if m != 0.0 {
                m = 1.0 / m;
            }
            return self.axis_modifier(from, &bf, direction) * m;
        }

        // Large Gear <-> Large Gear
        if is_large_to_large_gear(&bf, &bt, diff) {
            let source_diff = bf.axis.choose(diff);
            let target_diff = bt.axis.choose(diff);
            return if (source_diff > 0) ^ (target_diff > 0) { -1.0 } else { 1.0 };
        }

        // Gear <-> Large Gear
        if bf.is_large_cog() && bt.is_small_cog() && is_large_to_small_cog(&bf, &bt, diff) {
            return -2.0;
        }
        if bt.is_large_cog() && bf.is_small_cog() && is_large_to_small_cog(&bt, &bf, diff) {
            return -0.5;
        }

        // Gear <-> Gear
        if by_gears {
            if manhattan(diff) != 1 {
                return 0.0;
            }
            if bt.is_large_cog() {
                return 0.0;
            }
            if direction.axis() == bf.rotation_axis() {
                return 0.0;
            }
            if bf.rotation_axis() == bt.rotation_axis() {
                return -1.0;
            }
        }
        0.0
    }

    /// `getAxisModifier`.
    fn axis_modifier(&self, pos: Pos, block: &Block, direction: Direction) -> f32 {
        let Some(k) = self.w.kinetic(pos) else { return 1.0 };
        if !(k.has_source() || self.is_source(pos)) || !block.has_shaft_halves() {
            return 1.0;
        }
        let Some(source) = k.source else { return 1.0 };
        let src = Direction::nearest(sub(source, pos));
        if block.kind == Kind::Gearbox {
            return if direction.axis() == src.axis() {
                if direction == src { 1.0 } else { -1.0 }
            } else if direction.positive() == src.positive() {
                -1.0
            } else {
                1.0
            };
        }
        1.0
    }

    /// `getConveyedSpeed`.
    fn conveyed_speed(&self, from: Pos, to: Pos) -> f32 {
        self.theoretical(from) * self.rotation_speed_modifier(from, to)
    }

    /// `handleAdded`.
    fn handle_added(&mut self, pos: Pos) {
        if !self.w.loaded(pos) {
            return;
        }
        self.propagate_new_source(pos);
    }

    /// `propagateNewSource`.
    fn propagate_new_source(&mut self, current: Pos) {
        for neighbour in self.connected_neighbours(current) {
            if self.kin(current).is_none() {
                // Destroyed by an earlier neighbour.
                return;
            }
            if self.kin(neighbour).is_none() {
                continue;
            }
            let speed_of_current = self.theoretical(current);
            let speed_of_neighbour = self.theoretical(neighbour);
            let new_speed = self.conveyed_speed(current, neighbour);
            let opposite_speed = self.conveyed_speed(neighbour, current);

            if new_speed == 0.0 && opposite_speed == 0.0 {
                continue;
            }

            let incompatible = signum(new_speed) != signum(speed_of_neighbour)
                && new_speed != 0.0
                && speed_of_neighbour != 0.0;

            let too_fast = new_speed.abs() > MAX_ROTATION_SPEED || opposite_speed.abs() > MAX_ROTATION_SPEED;
            let flickering = self.kin(current).is_some_and(|k| k.flicker_tally > MAX_FLICKER_SCORE);
            if too_fast || flickering {
                self.destroy(current);
                return;
            }

            // Opposite directions
            if incompatible {
                self.destroy(current);
                return;
            }

            // Neighbour faster, overpower the incoming tree
            if opposite_speed.abs() > speed_of_current.abs() {
                let prev = self.speed(current);
                self.set_source(current, neighbour);
                let conveyed = self.conveyed_speed(neighbour, current);
                self.set_speed(current, conveyed);
                self.on_speed_changed(current, prev);
                self.w.send_data(current);
                self.propagate_new_source(current);
                return;
            }

            // Current faster, overpower the neighbours' tree
            if new_speed.abs() >= speed_of_neighbour.abs() {
                let cur = self.kin(current).unwrap_or_default();
                let nb = self.kin(neighbour).unwrap_or_default();
                // Do not overpower your own network -> cycle
                if !cur.has_network() || cur.network == nb.network {
                    let epsilon = speed_of_neighbour.abs() / 256.0 / 256.0;
                    if new_speed.abs() > speed_of_neighbour.abs() + epsilon {
                        self.destroy(current);
                    }
                    continue;
                }
                if cur.source == Some(neighbour) {
                    self.remove_source(current);
                }
                let prev = self.speed(neighbour);
                self.set_source(neighbour, current);
                let conveyed = self.conveyed_speed(current, neighbour);
                self.set_speed(neighbour, conveyed);
                self.on_speed_changed(neighbour, prev);
                self.w.send_data(neighbour);
                self.propagate_new_source(neighbour);
                continue;
            }

            if (self.theoretical(neighbour) - new_speed).abs() <= 1e-4 {
                continue;
            }
            let prev = self.speed(neighbour);
            self.set_speed(neighbour, new_speed);
            self.set_source(neighbour, current);
            self.on_speed_changed(neighbour, prev);
            self.w.send_data(neighbour);
            self.propagate_new_source(neighbour);
        }
    }

    /// `handleRemoved`, with the removed block entity's block and fields.
    fn handle_removed(&mut self, pos: Pos, block: &Block, removed: &Kinetic) {
        if removed.theoretical_speed() == 0.0 {
            return;
        }
        for neighbour in self.potential_neighbour_locations(pos, block) {
            let Some(nk) = self.kin(neighbour) else { continue };
            if nk.source != Some(pos) {
                continue;
            }
            self.propagate_missing_source(neighbour);
        }
    }

    /// `propagateMissingSource`.
    fn propagate_missing_source(&mut self, start: Pos) {
        let mut potential_new_sources: Vec<Pos> = Vec::new();
        let mut frontier: VecDeque<Pos> = VecDeque::from([start]);
        let missing_source = self.kin(start).and_then(|k| k.source);

        while let Some(pos) = frontier.pop_front() {
            if self.kin(pos).is_none() {
                continue;
            }
            self.remove_source(pos);
            self.w.send_data(pos);

            for neighbour in self.connected_neighbours(pos) {
                if Some(neighbour) == missing_source {
                    continue;
                }
                let Some(nk) = self.kin(neighbour) else { continue };
                if !nk.has_source() {
                    continue;
                }
                if nk.source != Some(pos) {
                    potential_new_sources.push(neighbour);
                    continue;
                }
                if self.is_source(neighbour) {
                    potential_new_sources.push(neighbour);
                }
                frontier.push_back(neighbour);
            }
        }

        for new_source in potential_new_sources {
            let has = self.kin(new_source).is_some_and(|k| k.has_source());
            if has || self.is_source(new_source) {
                self.propagate_new_source(new_source);
                return;
            }
        }
    }

    /// `isConnected`.
    pub fn is_connected(&self, from: Pos, to: Pos) -> bool {
        self.rotation_speed_modifier(from, to) != 0.0
    }

    /// `findConnectedNeighbour`.
    fn find_connected_neighbour(&self, current: Pos, neighbour: Pos) -> Option<Pos> {
        self.kin(neighbour)?;
        if !self.is_connected(current, neighbour) && !self.is_connected(neighbour, current) {
            return None;
        }
        Some(neighbour)
    }

    /// `getConnectedNeighbours`.
    pub fn connected_neighbours(&self, pos: Pos) -> Vec<Pos> {
        let Some(block) = self.w.block(pos) else { return Vec::new() };
        self.potential_neighbour_locations(pos, &block)
            .into_iter()
            .filter_map(|n| self.find_connected_neighbour(pos, n))
            .collect()
    }

    /// `getPotentialNeighbourLocations` (with `addPropagationLocations`).
    fn potential_neighbour_locations(&self, pos: Pos, block: &Block) -> Vec<Pos> {
        let mut out = Vec::new();
        if !self.w.loaded(pos) {
            return out;
        }
        for d in Direction::ALL {
            let r = add(pos, d.normal());
            if self.w.loaded(r) {
                out.push(r);
            }
        }
        // SimpleKineticBlockEntity: a large cog reaches every block diagonally
        // beside it.
        if block.is_large_cog() {
            for x in -1..=1 {
                for y in -1..=1 {
                    for z in -1..=1 {
                        if x * x + y * y + z * z == 2 {
                            out.push(add(pos, [x, y, z]));
                        }
                    }
                }
            }
            return out;
        }
        // `canPropagateDiagonally`: small cogs reach the large cogs beside them.
        if block.is_small_cog() {
            let axis = block.rotation_axis();
            for x in -1..=1 {
                for y in -1..=1 {
                    for z in -1..=1 {
                        let offset = [x, y, z];
                        if axis.choose(offset) != 0 {
                            continue;
                        }
                        if x * x + y * y + z * z != 2 {
                            continue;
                        }
                        out.push(add(pos, offset));
                    }
                }
            }
        }
        out
    }
}

/// `isLargeToLargeGear`.
fn is_large_to_large_gear(from: &Block, to: &Block, diff: Pos) -> bool {
    if !from.is_large_cog() || !to.is_large_cog() {
        return false;
    }
    if from.axis == to.axis {
        return false;
    }
    for axis in Axis::ALL {
        let d = axis.choose(diff);
        if axis == from.axis || axis == to.axis {
            if d == 0 {
                return false;
            }
        } else if d != 0 {
            return false;
        }
    }
    true
}

/// `isLargeToSmallCog`.
fn is_large_to_small_cog(from: &Block, to: &Block, diff: Pos) -> bool {
    let axis_from = from.axis;
    if axis_from != to.rotation_axis() {
        return false;
    }
    if axis_from.choose(diff) != 0 {
        return false;
    }
    Axis::ALL
        .into_iter()
        .filter(|a| *a != axis_from)
        .all(|a| a.choose(diff).abs() == 1)
}
