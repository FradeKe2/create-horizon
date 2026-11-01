// A kinetic block entity's fields (`KineticBlockEntity`,
// `GeneratingKineticBlockEntity`, `CreativeMotorBlockEntity`,
// `HandCrankBlockEntity`) and their saved form, under Create's own NBT keys.

use crate::geom::Pos;

/// Create's `CreativeMotorBlockEntity.DEFAULT_SPEED` and `MAX_SPEED`.
pub const MOTOR_DEFAULT_SPEED: i32 = 16;
pub const MOTOR_MAX_SPEED: i32 = 256;

#[derive(Clone, Debug, PartialEq)]
pub struct Kinetic {
    pub network: Option<i64>,
    pub source: Option<Pos>,
    pub network_dirty: bool,
    pub update_speed: bool,
    pub speed: f32,
    pub capacity: f32,
    pub stress: f32,
    pub over_stressed: bool,
    pub flicker_tally: i32,
    pub network_size: i32,
    pub validation_countdown: i32,
    pub last_stress_applied: f32,
    pub last_capacity_provided: f32,
    /// `GeneratingKineticBlockEntity.reActivateSource`.
    pub reactivate_source: bool,
    /// The creative motor's scroll value (rpm, -256..=256).
    pub motor_speed: i32,
    /// The hand crank's `inUse` and `backwards`.
    pub crank_in_use: i32,
    pub crank_backwards: bool,
    /// `SmartBlockEntity.initialized`: not saved, set on the first tick.
    pub initialized: bool,
}

impl Default for Kinetic {
    fn default() -> Self {
        Kinetic {
            network: None,
            source: None,
            network_dirty: false,
            // `updateSpeed = true` in the constructor.
            update_speed: true,
            speed: 0.0,
            capacity: 0.0,
            stress: 0.0,
            over_stressed: false,
            flicker_tally: 0,
            network_size: 0,
            validation_countdown: 0,
            last_stress_applied: 0.0,
            last_capacity_provided: 0.0,
            reactivate_source: false,
            motor_speed: MOTOR_DEFAULT_SPEED,
            crank_in_use: 0,
            crank_backwards: false,
            initialized: false,
        }
    }
}

/// A saved value, for the glue to turn into NBT.
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Float(f32),
    Int(i32),
    Long(i64),
    Bool(bool),
    Pos(Pos),
    Compound(Vec<(&'static str, Value)>),
}

impl Value {
    pub fn as_f32(&self) -> Option<f32> {
        match *self {
            Value::Float(f) => Some(f),
            Value::Int(i) => Some(i as f32),
            _ => None,
        }
    }
    pub fn as_i32(&self) -> Option<i32> {
        match *self {
            Value::Int(i) => Some(i),
            _ => None,
        }
    }
    pub fn as_i64(&self) -> Option<i64> {
        match *self {
            Value::Long(l) => Some(l),
            Value::Int(i) => Some(i as i64),
            _ => None,
        }
    }
    pub fn as_bool(&self) -> Option<bool> {
        match *self {
            Value::Bool(b) => Some(b),
            Value::Int(i) => Some(i != 0),
            _ => None,
        }
    }
}

/// What a lookup of saved fields reads.
pub trait Read {
    fn get(&self, key: &str) -> Option<Value>;
    fn sub(&self, key: &str) -> Option<Box<dyn Read + '_>>;
}

impl Kinetic {
    /// `KineticBlockEntity.write` (+ the motor's and crank's), as a list of
    /// fields. `client` is a client packet (`getUpdateTag`).
    pub fn write(&self, client: bool) -> Vec<(&'static str, Value)> {
        let mut out = vec![("Speed", Value::Float(self.speed))];
        if self.update_speed && !client {
            out.push(("NeedsSpeedUpdate", Value::Bool(true)));
        }
        if let Some(s) = self.source {
            out.push(("Source", Value::Pos(s)));
        }
        if let Some(id) = self.network {
            let mut net = vec![
                ("Id", Value::Long(id)),
                ("Stress", Value::Float(self.stress)),
                ("Capacity", Value::Float(self.capacity)),
                ("Size", Value::Int(self.network_size)),
            ];
            if self.last_stress_applied != 0.0 {
                net.push(("AddedStress", Value::Float(self.last_stress_applied)));
            }
            if self.last_capacity_provided != 0.0 {
                net.push(("AddedCapacity", Value::Float(self.last_capacity_provided)));
            }
            out.push(("Network", Value::Compound(net)));
        }
        out.push(("ScrollValue", Value::Int(self.motor_speed)));
        out.push(("InUse", Value::Int(self.crank_in_use)));
        out.push(("Backwards", Value::Bool(self.crank_backwards)));
        if client {
            // What the renderer needs: the speed the block actually turns at.
            out.push(("Overstressed", Value::Bool(self.over_stressed)));
        }
        out
    }

    /// `KineticBlockEntity.read`.
    pub fn read(r: &dyn Read) -> Kinetic {
        let mut k = Kinetic {
            // `clearKineticInformation`, then the saved fields.
            update_speed: false,
            ..Kinetic::default()
        };
        k.speed = r.get("Speed").and_then(|v| v.as_f32()).unwrap_or(0.0);
        if r.get("NeedsSpeedUpdate").and_then(|v| v.as_bool()).unwrap_or(false) {
            k.update_speed = true;
        }
        if let Some(Value::Pos(p)) = r.get("Source") {
            k.source = Some(p);
        }
        if let Some(net) = r.sub("Network") {
            k.network = net.get("Id").and_then(|v| v.as_i64());
            k.stress = net.get("Stress").and_then(|v| v.as_f32()).unwrap_or(0.0);
            k.capacity = net.get("Capacity").and_then(|v| v.as_f32()).unwrap_or(0.0);
            k.network_size = net.get("Size").and_then(|v| v.as_i32()).unwrap_or(0);
            k.last_stress_applied = net.get("AddedStress").and_then(|v| v.as_f32()).unwrap_or(0.0);
            k.last_capacity_provided = net.get("AddedCapacity").and_then(|v| v.as_f32()).unwrap_or(0.0);
            k.over_stressed = k.capacity < k.stress;
        }
        if let Some(v) = r.get("ScrollValue").and_then(|v| v.as_i32()) {
            k.motor_speed = v.clamp(-MOTOR_MAX_SPEED, MOTOR_MAX_SPEED);
        }
        k.crank_in_use = r.get("InUse").and_then(|v| v.as_i32()).unwrap_or(0);
        k.crank_backwards = r.get("Backwards").and_then(|v| v.as_bool()).unwrap_or(false);
        if let Some(o) = r.get("Overstressed").and_then(|v| v.as_bool()) {
            k.over_stressed = o;
        }
        // A fresh block entity (no saved fields) still needs its first speed update.
        if r.get("Speed").is_none() {
            k.update_speed = true;
        }
        k
    }

    /// `getTheoreticalSpeed`.
    pub fn theoretical_speed(&self) -> f32 {
        self.speed
    }

    /// `getSpeed`: nothing turns on an overstressed network.
    pub fn speed(&self) -> f32 {
        if self.over_stressed { 0.0 } else { self.speed }
    }

    pub fn has_source(&self) -> bool {
        self.source.is_some()
    }

    pub fn has_network(&self) -> bool {
        self.network.is_some()
    }
}
