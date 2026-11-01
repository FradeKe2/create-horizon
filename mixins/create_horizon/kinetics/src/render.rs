// How Create turns a kinetic block's model: `KineticBlockEntityRenderer`
// and `KineticBlockEntityVisual`.

use crate::blocks::Block;
use crate::geom::{Axis, Pos};

/// `KineticBlockEntityVisual.shouldOffset`: every other block along the
/// axis's plane is turned half a tooth, so neighbouring cogs mesh.
pub fn should_offset(axis: Axis, pos: Pos) -> bool {
    let x = if axis == Axis::X { 0 } else { pos[0] };
    let y = if axis == Axis::Y { 0 } else { pos[1] };
    let z = if axis == Axis::Z { 0 } else { pos[2] };
    (x + y + z) % 2 == 0
}

/// `KineticBlockEntityVisual.rotationOffset`, in degrees.
pub fn rotation_offset(block: &Block, axis: Axis, pos: Pos) -> f32 {
    if should_offset(axis, pos) {
        22.5
    } else if block.is_large_cog() {
        11.25
    } else {
        0.0
    }
}

/// `KineticBlockEntityRenderer.getAngleForBe`, in degrees: `time` is the
/// render time in ticks (`AnimationTickHolder.getRenderTime`), `speed` the
/// block's rpm (`getSpeed`).
pub fn angle(block: &Block, pos: Pos, speed: f32, time: f32) -> f32 {
    let axis = block.rotation_axis();
    let offset = rotation_offset(block, axis, pos);
    (time * speed * 3.0 / 10.0 + offset) % 360.0
}

/// `BracketedKineticBlockEntityRenderer.getAngleForLargeCogShaft`: the
/// shaft through a large cog keeps the small offset.
pub fn large_cog_shaft_angle(axis: Axis, pos: Pos, speed: f32, time: f32) -> f32 {
    let offset = if should_offset(axis, pos) { 22.5 } else { 0.0 };
    (time * speed * 3.0 / 10.0 + offset) % 360.0
}

/// The speedometer's needle (`SpeedGaugeBlockEntity.getDialTarget`): 0 to 1.
pub fn speed_dial(speed: f32) -> f32 {
    let speed = speed.abs();
    let medium = 30.0; // SpeedLevel MEDIUM (CKinetics.mediumSpeed)
    let fast = 100.0; // SpeedLevel FAST (CKinetics.fastSpeed)
    let max = crate::engine::MAX_ROTATION_SPEED;
    if speed == 0.0 {
        0.0
    } else if speed < medium {
        lerp(0.0, 0.45, speed / medium)
    } else if speed < fast {
        lerp(0.45, 0.75, (speed - medium) / (fast - medium))
    } else {
        lerp(0.75, 1.125, (speed - fast) / (max - fast))
    }
}

/// The stressometer's needle (`StressGaugeBlockEntity.updateFromNetwork`):
/// 0 to 1 of the network's capacity, 1.125 when overstressed.
pub fn stress_dial(stress: f32, capacity: f32, over_stressed: bool) -> f32 {
    if over_stressed {
        1.125
    } else if capacity == 0.0 {
        0.0
    } else {
        stress / capacity
    }
}

/// The stressometer's colour (`StressGaugeBlockEntity`): green to yellow to red.
pub fn stress_color(dial: f32) -> u32 {
    if dial <= 0.0 {
        0xFFFFFF
    } else if dial < 0.5 {
        mix(0x00FF00, 0xFFFF00, dial * 2.0)
    } else if dial < 1.0 {
        mix(0xFFFF00, 0xFF0000, dial * 2.0 - 1.0)
    } else {
        0xFF0000
    }
}

/// `Color.mixColors`.
pub fn mix(a: u32, b: u32, t: f32) -> u32 {
    let ch = |c: u32, s: u32| ((c >> s) & 0xFF) as f32;
    let m = |s: u32| ((ch(a, s) + (ch(b, s) - ch(a, s)) * t) as u32 & 0xFF) << s;
    m(16) | m(8) | m(0)
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}
