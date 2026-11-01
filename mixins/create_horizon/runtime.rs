// Create for Horizon: the glue between the hook points and the kinetics
// crate (`create_kinetics`, Create's rotation and stress system in Rust).
// Added to the game as `crate::game::create_mod`; the helpers call it.
//
// The block entities' saved compounds hold Create's own fields (`Speed`,
// `Source`, `Network`...). The kinetics read and write them through `Glue`,
// a `create_kinetics::World` over the game. Fields Create keeps only in
// memory (whether a block entity has initialized, its flicker score...) and
// each level's networks live in this module's statics, as Create keeps its
// networks in `TorquePropagator`.

use create_kinetics::state::{Read, Value};
use create_kinetics::{Axis, Block, Direction, Engine, Kind, Kinetic, Networks, Pos, World};
use horizon_client::containers::{BlockEntity, ModBlockEntity};
use horizon_world::nbt::{Compound, JavaString, Tag};
use std::collections::{HashMap, HashSet};
use std::sync::{LazyLock, Mutex};

/// The native mod's id (Options > Mods); its blocks are the `create` content mod's.
pub const ID: &str = "create_horizon";
pub const NAME: &str = "Create";
pub const VERSION: &str = "0.1.0";

type Key = (String, Pos);

#[derive(Default)]
struct State {
    /// Each level's kinetic networks (`TorquePropagator.networks`).
    networks: HashMap<String, Networks>,
    /// The last fields written for each kinetic block entity, so a removed
    /// one's are known, and the fields Create doesn't save.
    last: HashMap<Key, Kinetic>,
    /// Blocks the kinetics broke this tick, until the game removes them.
    destroyed: HashSet<Key>,
    /// Kinetic blocks the level replaced (`native_block_changed`): their
    /// level, position and block, for the next tick to detach.
    removed: Vec<(String, Pos, Block)>,
}

static STATE: LazyLock<Mutex<State>> = LazyLock::new(Default::default);

fn state() -> std::sync::MutexGuard<'static, State> {
    STATE.lock().unwrap_or_else(|e| e.into_inner())
}

/// Forget everything: the world closed.
pub fn reset() {
    *state() = State::default();
}

pub fn is_ours(block: &str) -> bool {
    Kind::of(block).is_some()
}

/// The kinetic block a block state names, from its name and properties.
pub fn block_of(name: &str, prop: impl Fn(&str) -> Option<String>) -> Option<Block> {
    let kind = Kind::of(name)?;
    if kind.has_facing() {
        let facing = prop("facing").and_then(|f| Direction::parse(&f)).unwrap_or(Direction::North);
        Some(Block::facing(kind, facing))
    } else if kind == Kind::Millstone {
        Some(Block::axial(kind, Axis::Y))
    } else {
        let axis = prop("axis").and_then(|a| Axis::parse(&a)).unwrap_or(Axis::Y);
        Some(Block::axial(kind, axis))
    }
}

pub fn dimension_id(d: horizon_world::level::Dimension) -> &'static str {
    match d {
        horizon_world::level::Dimension::Overworld => "minecraft:overworld",
        horizon_world::level::Dimension::Nether => "minecraft:the_nether",
        horizon_world::level::Dimension::End => "minecraft:the_end",
    }
}

// ---- NBT ------------------------------------------------------------------

fn key(s: &str) -> JavaString {
    JavaString::from(s)
}

fn to_tag(v: &Value) -> Tag {
    match v {
        Value::Float(f) => Tag::Float(f.to_bits()),
        Value::Int(i) => Tag::Int(*i),
        Value::Long(l) => Tag::Long(*l),
        Value::Bool(b) => Tag::Byte(*b as i8),
        // NbtUtils.writeBlockPos: an int array.
        Value::Pos(p) => Tag::IntArray(p.to_vec()),
        Value::Compound(c) => Tag::Compound(c.iter().map(|(k, v)| (key(k), to_tag(v))).collect()),
    }
}

fn from_tag(t: &Tag) -> Option<Value> {
    Some(match t {
        Tag::Float(bits) => Value::Float(f32::from_bits(*bits)),
        Tag::Double(bits) => Value::Float(f64::from_bits(*bits) as f32),
        Tag::Int(i) => Value::Int(*i),
        Tag::Short(s) => Value::Int(*s as i32),
        Tag::Byte(b) => Value::Bool(*b != 0),
        Tag::Long(l) => Value::Long(*l),
        Tag::IntArray(a) if a.len() == 3 => Value::Pos([a[0], a[1], a[2]]),
        _ => return None,
    })
}

struct Nbt<'a>(&'a Compound);

impl Read for Nbt<'_> {
    fn get(&self, k: &str) -> Option<Value> {
        self.0.get(&key(k)).and_then(from_tag)
    }
    fn sub(&self, k: &str) -> Option<Box<dyn Read + '_>> {
        match self.0.get(&key(k)) {
            Some(Tag::Compound(c)) => Some(Box::new(Nbt(c))),
            _ => None,
        }
    }
}

/// The kinetic fields of a saved compound.
pub fn read(nbt: &Compound) -> Kinetic {
    Kinetic::read(&Nbt(nbt))
}

/// Writes the kinetic fields into a saved compound, keeping its other keys.
pub fn write(nbt: &mut Compound, k: &Kinetic) {
    for name in ["Speed", "NeedsSpeedUpdate", "Source", "Network", "ScrollValue", "InUse", "Backwards"] {
        nbt.remove(&key(name));
    }
    for (name, v) in k.write(false) {
        nbt.insert(key(name), to_tag(&v));
    }
}

/// What clients get (`getUpdateTag`).
pub fn update_tag(entity: &ModBlockEntity) -> Compound {
    let k = read(&entity.nbt);
    k.write(true).iter().map(|(n, v)| (key(n), to_tag(v))).collect()
}

/// Copies the fields Create keeps only in memory from the last copy.
fn with_transient(mut k: Kinetic, last: Option<&Kinetic>) -> Kinetic {
    if let Some(l) = last {
        k.initialized = l.initialized;
        k.network_dirty = l.network_dirty;
        k.flicker_tally = l.flicker_tally;
        k.validation_countdown = l.validation_countdown;
        k.reactivate_source = l.reactivate_source;
    }
    k
}

// ---- the world the kinetics see ---------------------------------------------

/// The game as the kinetics see it, during a server hook in the level `dim`.
/// `current` is the block entity ticking now, which is out of the world.
struct Glue<'g, 'a> {
    game: &'g mut super::Game<'a>,
    dim: String,
    current: Option<(Pos, &'g mut ModBlockEntity)>,
    st: &'g mut State,
    synced: Vec<Pos>,
    destroys: Vec<Pos>,
}

impl World for Glue<'_, '_> {
    fn block(&self, pos: Pos) -> Option<Block> {
        if self.st.destroyed.contains(&(self.dim.clone(), pos)) {
            return None;
        }
        let name = self.game.native_block(pos);
        block_of(&name, |k| self.game.native_block_property(pos, k))
    }

    fn loaded(&self, pos: Pos) -> bool {
        self.game.world.loaded(pos[0], pos[2])
    }

    fn kinetic(&self, pos: Pos) -> Option<Kinetic> {
        let k = (self.dim.clone(), pos);
        if self.st.destroyed.contains(&k) {
            return None;
        }
        let saved = match &self.current {
            Some((p, e)) if *p == pos => read(&e.nbt),
            _ => match self.game.world.block_entities.get(&(pos[0], pos[1], pos[2])) {
                Some(BlockEntity::Mod(m)) if is_ours(&m.id) => read(&m.nbt),
                _ => return None,
            },
        };
        Some(with_transient(saved, self.st.last.get(&k)))
    }

    fn set_kinetic(&mut self, pos: Pos, k: Kinetic) {
        let written = match &mut self.current {
            Some((p, e)) if *p == pos => {
                write(&mut e.nbt, &k);
                true
            }
            _ => self
                .game
                .native_with_mod_block_entity(pos, |m| {
                    if is_ours(&m.id) {
                        write(&mut m.nbt, &k);
                        true
                    } else {
                        false
                    }
                })
                .unwrap_or(false),
        };
        if written {
            self.st.last.insert((self.dim.clone(), pos), k);
        }
    }

    fn send_data(&mut self, pos: Pos) {
        if !self.synced.contains(&pos) {
            self.synced.push(pos);
        }
    }

    fn destroy_block(&mut self, pos: Pos) {
        let k = (self.dim.clone(), pos);
        self.st.last.remove(&k);
        self.st.destroyed.insert(k);
        self.destroys.push(pos);
    }
}

impl<'g, 'a> Glue<'g, 'a> {
    /// Runs `f` with the kinetics over the game, then sends what changed and
    /// breaks what they broke.
    fn run<R>(
        game: &'g mut super::Game<'a>,
        current: Option<(Pos, &'g mut ModBlockEntity)>,
        st: &'g mut State,
        f: impl FnOnce(&mut Engine<'_, Glue<'g, 'a>>) -> R,
    ) -> R {
        let dim = game.native_dimension().to_owned();
        let mut nets = st.networks.remove(&dim).unwrap_or_default();
        let mut glue = Glue { game, dim: dim.clone(), current, st, synced: Vec::new(), destroys: Vec::new() };
        let r = {
            let mut engine = Engine::new(&mut glue, &mut nets);
            f(&mut engine)
        };
        glue.st.networks.insert(dim, nets);
        glue.finish();
        r
    }

    fn finish(self) {
        let Glue { game, dim, current, synced, destroys, .. } = self;
        let current = current.map(|(p, _)| p);
        for pos in synced {
            // The ticking one goes out with its tick (`native_block_entity_update_tag`).
            if Some(pos) != current {
                game.native_block_entity_sync(pos);
            }
        }
        if let Some(pos) = current {
            game.native_block_entity_sync(pos);
        }
        for pos in destroys {
            // Level.destroyBlock(pos, true): the block breaks and drops.
            let cmd = format!("execute in {dim} run setblock {} {} {} minecraft:air destroy", pos[0], pos[1], pos[2]);
            if !game.native_run_command(&cmd)
                && let Some(air) = game.pack.assets.registry.default_state("minecraft:air")
            {
                game.native_set_block(&dim, pos, air, horizon_world::level::flags::UPDATE_ALL);
            }
        }
    }
}

// ---- hooks ------------------------------------------------------------------

/// `native_block_entity`: each kinetic block gets its block entity.
pub fn new_block_entity(block: &str, entity: &mut Option<(String, Compound)>) {
    if entity.is_none() && is_ours(block) {
        *entity = Some((block.to_owned(), Compound::new()));
    }
}

/// Detaches the kinetic blocks the level replaced in `dim`.
fn drain_removed(game: &mut super::Game<'_>, st: &mut State, dim: &str) {
    if st.removed.is_empty() {
        return;
    }
    let (now, later): (Vec<_>, Vec<_>) = std::mem::take(&mut st.removed).into_iter().partition(|r| r.0 == dim);
    st.removed = later;
    for (_, pos, block) in now {
        let k = (dim.to_owned(), pos);
        if st.destroyed.remove(&k) {
            // The kinetics broke it and already detached it.
            continue;
        }
        let Some(last) = st.last.remove(&k) else { continue };
        Glue::run(game, None, st, |e| e.removed(pos, &block, &last));
    }
}

/// `native_block_entity_tick`.
pub(in crate::game) fn tick(game: &mut super::Game<'_>, pos: Pos, entity: &mut ModBlockEntity) {
    if !is_ours(&entity.id) {
        return;
    }
    let mut st = state();
    let dim = game.native_dimension().to_owned();
    drain_removed(game, &mut st, &dim);
    Glue::run(game, Some((pos, entity)), &mut st, |e| e.tick(pos));
}

/// `native_level_tick_pre`: detach what was removed since.
pub(in crate::game) fn level_tick(game: &mut super::Game<'_>, dimension: &str) {
    if game.native_dimension() != dimension {
        return;
    }
    let mut st = state();
    drain_removed(game, &mut st, dimension);
}

/// `native_block_changed`: a kinetic block left its place, or changed its
/// axis. Read-only here; the next tick detaches it.
pub fn block_changed(dim: &str, pos: Pos, old: Option<Block>, new: Option<Block>) {
    let Some(old) = old else { return };
    if new == Some(old) {
        return;
    }
    state().removed.push((dim.to_owned(), pos, old));
}

/// `native_right_click_block`: a player turns the hand crank or sets the
/// creative motor (Create scrolls its value; here a click raises it and a
/// sneaking click lowers it).
pub(in crate::game) fn right_click(game: &mut super::Game<'_>, pos: Pos, sneaking: bool) -> Option<String> {
    let name = game.native_block(pos);
    let kind = Kind::of(&name)?;
    let mut st = state();
    match kind {
        Kind::HandCrank => {
            Glue::run(game, None, &mut st, |e| e.crank(pos, sneaking));
            None
        }
        Kind::CreativeMotor => {
            let current = match game.world.block_entities.get(&(pos[0], pos[1], pos[2])) {
                Some(BlockEntity::Mod(m)) if is_ours(&m.id) => read(&m.nbt).motor_speed,
                _ => return None,
            };
            let next = step_motor(current, !sneaking);
            Glue::run(game, None, &mut st, |e| e.set_motor_speed(pos, next));
            Some(format!("Rotation speed: {next} RPM"))
        }
        _ => None,
    }
}

/// Create's item application (`create:item_application`): andesite alloy
/// used on a stripped log or wood makes andesite casing. True when it did.
pub(in crate::game) fn apply_item(game: &mut super::Game<'_>, pos: Pos) -> bool {
    let Some((held, _, _)) = game.native_held() else { return false };
    if held != "create:andesite_alloy" || !is_stripped_log(&game.native_block(pos)) {
        return false;
    }
    let Some(casing) = game.pack.assets.registry.default_state("create:andesite_casing") else {
        return false;
    };
    let dim = game.native_dimension();
    if !game.native_set_block(dim, pos, casing, horizon_world::level::flags::UPDATE_ALL) {
        return false;
    }
    // Survival and adventure use the alloy up; creative keeps it.
    if game.mode_id() != 1 {
        game.native_take_items("create:andesite_alloy", 1);
    }
    true
}

/// `#c:stripped_logs`, as NeoForge fills it.
pub fn is_stripped_log(name: &str) -> bool {
    let Some(rest) = name.strip_prefix("minecraft:stripped_") else { return false };
    rest == "bamboo_block" || ["_log", "_wood", "_stem", "_hyphae"].iter().any(|s| rest.ends_with(s))
}

/// The creative motor's next value: powers of two up to 256 rpm, either way.
pub fn step_motor(rpm: i32, up: bool) -> i32 {
    const STEPS: [i32; 19] = [-256, -128, -64, -32, -16, -8, -4, -2, -1, 0, 1, 2, 4, 8, 16, 32, 64, 128, 256];
    let i = STEPS.iter().position(|s| *s >= rpm).unwrap_or(STEPS.len() - 1);
    let i = if up {
        if STEPS[i] > rpm { i } else { (i + 1).min(STEPS.len() - 1) }
    } else {
        i.saturating_sub(1)
    };
    STEPS[i]
}

/// `native_state_for_placement`: the axis or facing a kinetic block takes,
/// as Create's `getStateForPlacement` picks it from the click.
pub fn placement(
    name: &str,
    face: Direction,
    yaw: f32,
    pitch: f32,
    sneaking: bool,
) -> Option<(&'static str, &'static str)> {
    let kind = Kind::of(name)?;
    match kind {
        // DirectionalKineticBlock: the nearest looking direction, reversed
        // unless sneaking.
        Kind::CreativeMotor => {
            let look = looking(yaw, pitch);
            Some(("facing", if sneaking { look.name() } else { look.opposite().name() }))
        }
        // HandCrankBlock: the clicked face.
        Kind::HandCrank => Some(("facing", face.name())),
        Kind::Millstone => None,
        // RotatedPillarKineticBlock / CogWheelBlock: the clicked face's axis.
        _ => Some(("axis", face.axis().name())),
    }
}

/// `BlockPlaceContext.getNearestLookingDirection`.
pub fn looking(yaw: f32, pitch: f32) -> Direction {
    let (y, p) = (yaw.to_radians(), pitch.to_radians());
    let v = [-y.sin() * p.cos(), -p.sin(), y.cos() * p.cos()];
    let mut best = Direction::North;
    let mut dot = f32::MIN;
    for d in Direction::ALL {
        let n = d.normal();
        let x = v[0] * n[0] as f32 + v[1] * n[1] as f32 + v[2] * n[2] as f32;
        if x > dot {
            dot = x;
            best = d;
        }
    }
    best
}

// ---- drawing ----------------------------------------------------------------

/// Turns, oriented and textured parts: what `native_block_entity_submit` draws.
pub struct Part {
    /// A box in texels (16 a block), along +y.
    pub from: [f32; 3],
    pub to: [f32; 3],
    /// Sprites for its sides and ends.
    pub side: &'static str,
    pub end: &'static str,
    /// Degrees about its axis before it turns (the teeth's spacing).
    pub turn: f32,
}

const SHAFT: &str = "create:block/axis";
const SHAFT_END: &str = "create:block/axis_top";
const COG: &str = "create:block/cogwheel";
const COG_SIDE: &str = "create:block/cogwheel_side";
const LARGE_COG: &str = "create:block/large_cogwheel";
const CRANK: &str = "create:block/hand_crank";
const NEEDLE: &str = "create:block/gauge_needle";

fn shaft_part(from_y: f32, to_y: f32) -> Part {
    Part { from: [6.0, from_y, 6.0], to: [10.0, to_y, 10.0], side: SHAFT, end: SHAFT_END, turn: 0.0 }
}

/// A cog of `teeth` teeth: an octagonal disc and its teeth, out to `radius`.
fn cog_parts(out: &mut Vec<Part>, radius: f32, teeth: usize, face: &'static str) {
    let r = radius - 2.0;
    for turn in [0.0, 45.0] {
        out.push(Part { from: [8.0 - r, 6.0, 8.0 - r], to: [8.0 + r, 10.0, 8.0 + r], side: COG_SIDE, end: face, turn });
    }
    for i in 0..teeth {
        let turn = i as f32 * 360.0 / teeth as f32;
        out.push(Part {
            from: [7.0, 6.5, 8.0 + r - 0.5],
            to: [9.0, 9.5, 8.0 + radius],
            side: COG_SIDE,
            end: COG_SIDE,
            turn,
        });
    }
}

/// The turning parts of a kinetic block, along +y, and the static ones.
pub fn turning_parts(kind: Kind) -> Vec<Part> {
    let mut out = Vec::new();
    match kind {
        Kind::Shaft | Kind::Speedometer | Kind::Stressometer | Kind::MechanicalPress => {
            out.push(shaft_part(0.0, 16.0))
        }
        Kind::Cogwheel | Kind::Millstone => {
            out.push(shaft_part(0.0, 16.0));
            cog_parts(&mut out, 7.0, 14, COG);
        }
        Kind::LargeCogwheel => {
            out.push(shaft_part(0.0, 16.0));
            cog_parts(&mut out, 12.0, 32, LARGE_COG);
        }
        Kind::CreativeMotor => out.push(shaft_part(8.0, 16.0)),
        Kind::HandCrank => {
            out.push(shaft_part(0.0, 6.0));
            out.push(Part { from: [7.0, 5.0, 2.0], to: [9.0, 7.0, 14.0], side: CRANK, end: CRANK, turn: 0.0 });
            out.push(Part { from: [7.0, 5.0, 12.0], to: [9.0, 12.0, 14.0], side: CRANK, end: CRANK, turn: 0.0 });
        }
        Kind::Gearbox => {}
    }
    out
}

/// The rotation that takes +y to `axis` (and to its positive end).
pub fn orient(axis: Axis) -> Option<([f32; 3], f32)> {
    match axis {
        Axis::Y => None,
        // +y to +x: a quarter turn about -z.
        Axis::X => Some(([0.0, 0.0, 1.0], -90.0)),
        // +y to +z: a quarter turn about +x.
        Axis::Z => Some(([1.0, 0.0, 0.0], 90.0)),
    }
}

/// The rotation that takes +y to `facing`.
pub fn orient_facing(facing: Direction) -> ([f32; 3], f32) {
    match facing {
        Direction::Up => ([1.0, 0.0, 0.0], 0.0),
        Direction::Down => ([1.0, 0.0, 0.0], 180.0),
        Direction::South => ([1.0, 0.0, 0.0], 90.0),
        Direction::North => ([1.0, 0.0, 0.0], -90.0),
        Direction::East => ([0.0, 0.0, 1.0], -90.0),
        Direction::West => ([0.0, 0.0, 1.0], 90.0),
    }
}

/// What the client knows of a kinetic block entity.
pub struct Shown {
    pub speed: f32,
    pub source: Option<Pos>,
    pub stress: f32,
    pub capacity: f32,
    pub over_stressed: bool,
}

pub fn shown(entity: &ModBlockEntity) -> Shown {
    let k = read(&entity.nbt);
    Shown { speed: k.speed(), source: k.source, stress: k.stress, capacity: k.capacity, over_stressed: k.over_stressed }
}

/// The gearbox's halves: each side's direction and the factor of its turn
/// (`GearboxRenderer`).
pub fn gearbox_halves(axis: Axis, pos: Pos, source: Option<Pos>, speed: f32) -> Vec<(Direction, f32)> {
    let src = source.filter(|_| speed != 0.0).map(|s| Direction::nearest([s[0] - pos[0], s[1] - pos[1], s[2] - pos[2]]));
    Direction::ALL
        .into_iter()
        .filter(|d| d.axis() != axis)
        .map(|d| {
            let f = match src {
                Some(s) if s.axis() == d.axis() => {
                    if s == d { 1.0 } else { -1.0 }
                }
                Some(s) if s.positive() == d.positive() => -1.0,
                _ => 1.0,
            };
            (d, f)
        })
        .collect()
}

pub const NEEDLE_SPRITE: &str = NEEDLE;

type Draw = super::native_block_entity_draw::BlockEntityDraw;

const CENTER: [f32; 3] = [0.5, 0.5, 0.5];

fn draw_part(game: &super::Game<'_>, draw: &mut Draw, part: &Part, tint: u32) {
    use super::native_block_entity_draw::CubeFace;
    let side = Some(CubeFace { sprite: part.side, uv: None });
    let end = Some(CubeFace { sprite: part.end, uv: None });
    draw.push();
    if part.turn != 0.0 {
        draw.rotate_about([0.0, 1.0, 0.0], part.turn, CENTER);
    }
    game.native_draw_cube(draw, part.from, part.to, [end, end, side, side, side, side], tint);
    draw.pop();
}

/// Draws `parts` (along +y) turned `angle` degrees about `axis`, after
/// `orient` takes +y where they belong.
fn draw_turning(game: &super::Game<'_>, draw: &mut Draw, parts: &[Part], axis: Axis, angle: f32, orient: Option<([f32; 3], f32)>) {
    draw.push();
    draw.rotate_about(axis.unit(), angle, CENTER);
    if let Some((about, deg)) = orient {
        draw.rotate_about(about, deg, CENTER);
    }
    for p in parts {
        draw_part(game, draw, p, 0xFFFFFF);
    }
    draw.pop();
}

/// The render time in ticks (`AnimationTickHolder.getRenderTime`), wrapped
/// where whole-rpm turns repeat so it stays precise.
fn render_time(draw: &Draw) -> f32 {
    (draw.game_time.rem_euclid(72_000)) as f32 + draw.partial
}

/// `native_block_entity_submit`: Create's kinetic renderers.
pub(in crate::game) fn submit(
    game: &super::Game<'_>,
    pos: Pos,
    name: &str,
    state: horizon_world::registry::BlockStateId,
    entity: &ModBlockEntity,
    draw: &mut Draw,
) {
    use create_kinetics::render;
    let Some(block) = block_of(name, |k| game.native_block_state_property(state, k).map(str::to_owned)) else {
        return;
    };
    let s = shown(entity);
    let time = render_time(draw);
    let axis = block.rotation_axis();
    match block.kind {
        Kind::Gearbox => {
            let half = [shaft_part(8.0, 16.0)];
            for (d, factor) in gearbox_halves(axis, pos, s.source, s.speed) {
                let angle = render::angle(&Block::axial(Kind::Shaft, d.axis()), pos, s.speed, time) * factor;
                draw_turning(game, draw, &half, d.axis(), angle, Some(orient_facing(d)));
            }
        }
        Kind::CreativeMotor | Kind::HandCrank => {
            let facing = block.facing.unwrap_or(Direction::Up);
            let angle = render::angle(&block, pos, s.speed, time);
            draw_turning(game, draw, &turning_parts(block.kind), axis, angle, Some(orient_facing(facing)));
        }
        Kind::LargeCogwheel => {
            let angle = render::angle(&block, pos, s.speed, time);
            let parts = turning_parts(block.kind);
            let (shaft, cog) = parts.split_at(1);
            let shaft_angle = render::large_cog_shaft_angle(axis, pos, s.speed, time);
            draw_turning(game, draw, shaft, axis, shaft_angle, orient(axis));
            draw_turning(game, draw, cog, axis, angle, orient(axis));
        }
        Kind::Speedometer | Kind::Stressometer => {
            let angle = render::angle(&block, pos, s.speed, time);
            draw_turning(game, draw, &turning_parts(block.kind), axis, angle, orient(axis));
            let (dial, tint) = if block.kind == Kind::Speedometer {
                (render::speed_dial(s.speed), 0xFFFFFF)
            } else {
                let d = render::stress_dial(s.stress, s.capacity, s.over_stressed);
                (d, render::stress_color(d))
            };
            draw_needles(game, draw, axis, dial, tint);
        }
        _ => {
            let angle = render::angle(&block, pos, s.speed, time);
            draw_turning(game, draw, &turning_parts(block.kind), axis, angle, orient(axis));
        }
    }
}

/// A gauge's needles, on the four faces along its axis: from straight down
/// at 0 round to 1.125 (`GaugeRenderer`: -90 to 180 degrees of the dial).
fn draw_needles(game: &super::Game<'_>, draw: &mut Draw, axis: Axis, dial: f32, tint: u32) {
    let needle = Part { from: [7.5, 15.9, 3.0], to: [8.5, 16.4, 8.5], side: NEEDLE, end: NEEDLE, turn: 0.0 };
    let degrees = -90.0 + dial.clamp(0.0, 1.125) * 240.0;
    for face in Direction::ALL.into_iter().filter(|d| d.axis() != axis) {
        draw.push();
        let (about, deg) = orient_facing(face);
        draw.rotate_about(about, deg, CENTER);
        draw.rotate_about([0.0, 1.0, 0.0], degrees, CENTER);
        draw_part(game, draw, &needle, tint);
        draw.pop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn motor_steps() {
        assert_eq!(step_motor(16, true), 32);
        assert_eq!(step_motor(16, false), 8);
        assert_eq!(step_motor(256, true), 256);
        assert_eq!(step_motor(1, false), 0);
        assert_eq!(step_motor(0, false), -1);
        assert_eq!(step_motor(20, true), 32);
        assert_eq!(step_motor(20, false), 16);
    }

    #[test]
    fn stripped_logs() {
        assert!(is_stripped_log("minecraft:stripped_oak_log"));
        assert!(is_stripped_log("minecraft:stripped_crimson_hyphae"));
        assert!(is_stripped_log("minecraft:stripped_bamboo_block"));
        assert!(!is_stripped_log("minecraft:oak_log"));
    }

    #[test]
    fn looking_directions() {
        assert_eq!(looking(0.0, 0.0), Direction::South);
        assert_eq!(looking(90.0, 0.0), Direction::West);
        assert_eq!(looking(0.0, 90.0), Direction::Down);
    }

    #[test]
    fn nbt_round_trip() {
        let mut k = Kinetic::default();
        k.speed = 32.0;
        k.source = Some([1, 2, 3]);
        k.network = Some(42);
        k.stress = 5.0;
        k.capacity = 9.0;
        k.update_speed = false;
        let mut nbt = Compound::new();
        nbt.insert(key("Other"), Tag::Int(7));
        write(&mut nbt, &k);
        let back = read(&nbt);
        assert_eq!(back.speed, 32.0);
        assert_eq!(back.source, Some([1, 2, 3]));
        assert_eq!(back.network, Some(42));
        assert_eq!(nbt.get(&key("Other")), Some(&Tag::Int(7)));
    }
}
