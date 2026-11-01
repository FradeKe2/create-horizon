// The kinetic blocks and what Create's `IRotate`, `ICogWheel` and its stress
// config say of each.

use crate::geom::{Axis, Direction};

/// The kinetic blocks of this port.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Kind {
    Shaft,
    Cogwheel,
    LargeCogwheel,
    Gearbox,
    CreativeMotor,
    HandCrank,
    Speedometer,
    Stressometer,
    /// A consumer used to load networks: `create:mechanical_press`'s stress
    /// impact, with no processing yet.
    MechanicalPress,
    Millstone,
}

impl Kind {
    pub const ALL: [Kind; 10] = [
        Kind::Shaft,
        Kind::Cogwheel,
        Kind::LargeCogwheel,
        Kind::Gearbox,
        Kind::CreativeMotor,
        Kind::HandCrank,
        Kind::Speedometer,
        Kind::Stressometer,
        Kind::MechanicalPress,
        Kind::Millstone,
    ];

    /// The block's id, as in Create.
    pub fn id(self) -> &'static str {
        match self {
            Kind::Shaft => "create:shaft",
            Kind::Cogwheel => "create:cogwheel",
            Kind::LargeCogwheel => "create:large_cogwheel",
            Kind::Gearbox => "create:gearbox",
            Kind::CreativeMotor => "create:creative_motor",
            Kind::HandCrank => "create:hand_crank",
            Kind::Speedometer => "create:speedometer",
            Kind::Stressometer => "create:stressometer",
            Kind::MechanicalPress => "create:mechanical_press",
            Kind::Millstone => "create:millstone",
        }
    }

    pub fn of(id: &str) -> Option<Kind> {
        Kind::ALL.into_iter().find(|k| k.id() == id)
    }

    /// Whether the block's state has `facing` (a direction) rather than `axis`.
    pub fn has_facing(self) -> bool {
        matches!(self, Kind::CreativeMotor | Kind::HandCrank)
    }

    /// `CStress` impact (stress units per rpm). Create's defaults.
    pub fn stress_impact(self) -> f32 {
        match self {
            Kind::MechanicalPress => 8.0,
            Kind::Millstone => 4.0,
            _ => 0.0,
        }
    }

    /// `CStress` capacity (stress units per rpm). Create's defaults.
    pub fn stress_capacity(self) -> f32 {
        match self {
            Kind::CreativeMotor => 16384.0,
            Kind::HandCrank => 8.0,
            _ => 0.0,
        }
    }

    pub fn is_generator(self) -> bool {
        matches!(self, Kind::CreativeMotor | Kind::HandCrank)
    }
}

/// A kinetic block in the world: its kind and the state properties the
/// kinetics read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Block {
    pub kind: Kind,
    /// `axis` for pillar blocks; the facing's axis for directional ones.
    pub axis: Axis,
    /// `facing`, for directional blocks.
    pub facing: Option<Direction>,
}

impl Block {
    pub fn axial(kind: Kind, axis: Axis) -> Block {
        Block { kind, axis, facing: None }
    }

    pub fn facing(kind: Kind, facing: Direction) -> Block {
        Block { kind, axis: facing.axis(), facing: Some(facing) }
    }

    /// `IRotate.getRotationAxis`.
    pub fn rotation_axis(&self) -> Axis {
        self.axis
    }

    /// `IRotate.hasShaftTowards`.
    pub fn has_shaft_towards(&self, face: Direction) -> bool {
        match self.kind {
            // AbstractShaftBlock, CogWheelBlock (a shaft runs through it), the
            // gauges (`DirectionalAxisKineticBlock`), the press and millstone
            // (shaft along their axis).
            Kind::Shaft | Kind::Cogwheel | Kind::LargeCogwheel | Kind::Speedometer | Kind::Stressometer => {
                face.axis() == self.axis
            }
            Kind::MechanicalPress => face.axis() == self.axis,
            // The millstone takes its input from below (`face == DOWN`).
            Kind::Millstone => face == Direction::Down,
            // GearboxBlock: every face off its axis.
            Kind::Gearbox => face.axis() != self.axis,
            // CreativeMotorBlock: `face == facing`.
            Kind::CreativeMotor => Some(face) == self.facing,
            // HandCrankBlock: `face == facing.getOpposite()`.
            Kind::HandCrank => Some(face) == self.facing.map(Direction::opposite),
        }
    }

    /// `ICogWheel.isSmallCog`.
    pub fn is_small_cog(&self) -> bool {
        matches!(self.kind, Kind::Cogwheel | Kind::Millstone)
    }

    /// `ICogWheel.isLargeCog`.
    pub fn is_large_cog(&self) -> bool {
        self.kind == Kind::LargeCogwheel
    }

    /// `DirectionalShaftHalvesBlockEntity`: blocks whose halves turn by where
    /// their source is.
    pub fn has_shaft_halves(&self) -> bool {
        self.kind == Kind::Gearbox
    }
}
