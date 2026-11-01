// Positions, axes and directions, as Minecraft's `BlockPos`, `Direction.Axis`
// and `Direction` are used by Create's kinetics.

/// A block position.
pub type Pos = [i32; 3];

pub fn add(a: Pos, b: Pos) -> Pos {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

pub fn sub(a: Pos, b: Pos) -> Pos {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

pub fn manhattan(d: Pos) -> i32 {
    d[0].abs() + d[1].abs() + d[2].abs()
}

/// `BlockPos.asLong`, which Create uses as a network's id.
pub fn as_long(p: Pos) -> i64 {
    const XZ_BITS: u32 = 26;
    const Y_BITS: u32 = 12;
    let x = (p[0] as i64) & ((1 << XZ_BITS) - 1);
    let y = (p[1] as i64) & ((1 << Y_BITS) - 1);
    let z = (p[2] as i64) & ((1 << XZ_BITS) - 1);
    (x << (XZ_BITS + Y_BITS)) | (z << Y_BITS) | y
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Axis {
    X,
    Y,
    Z,
}

impl Axis {
    pub const ALL: [Axis; 3] = [Axis::X, Axis::Y, Axis::Z];

    /// `Axis.choose(x, y, z)`.
    pub fn choose(self, p: Pos) -> i32 {
        match self {
            Axis::X => p[0],
            Axis::Y => p[1],
            Axis::Z => p[2],
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Axis::X => "x",
            Axis::Y => "y",
            Axis::Z => "z",
        }
    }

    pub fn parse(s: &str) -> Option<Axis> {
        match s {
            "x" => Some(Axis::X),
            "y" => Some(Axis::Y),
            "z" => Some(Axis::Z),
            _ => None,
        }
    }

    pub fn unit(self) -> [f32; 3] {
        match self {
            Axis::X => [1.0, 0.0, 0.0],
            Axis::Y => [0.0, 1.0, 0.0],
            Axis::Z => [0.0, 0.0, 1.0],
        }
    }
}

/// Minecraft's `Direction`, in its order (down, up, north, south, west, east).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Direction {
    Down,
    Up,
    North,
    South,
    West,
    East,
}

impl Direction {
    pub const ALL: [Direction; 6] = [
        Direction::Down,
        Direction::Up,
        Direction::North,
        Direction::South,
        Direction::West,
        Direction::East,
    ];

    pub fn normal(self) -> Pos {
        match self {
            Direction::Down => [0, -1, 0],
            Direction::Up => [0, 1, 0],
            Direction::North => [0, 0, -1],
            Direction::South => [0, 0, 1],
            Direction::West => [-1, 0, 0],
            Direction::East => [1, 0, 0],
        }
    }

    pub fn axis(self) -> Axis {
        match self {
            Direction::Down | Direction::Up => Axis::Y,
            Direction::North | Direction::South => Axis::Z,
            Direction::West | Direction::East => Axis::X,
        }
    }

    /// `getAxisDirection() == POSITIVE`.
    pub fn positive(self) -> bool {
        matches!(self, Direction::Up | Direction::South | Direction::East)
    }

    pub fn opposite(self) -> Direction {
        match self {
            Direction::Down => Direction::Up,
            Direction::Up => Direction::Down,
            Direction::North => Direction::South,
            Direction::South => Direction::North,
            Direction::West => Direction::East,
            Direction::East => Direction::West,
        }
    }

    pub fn from_axis(axis: Axis, positive: bool) -> Direction {
        match (axis, positive) {
            (Axis::X, true) => Direction::East,
            (Axis::X, false) => Direction::West,
            (Axis::Y, true) => Direction::Up,
            (Axis::Y, false) => Direction::Down,
            (Axis::Z, true) => Direction::South,
            (Axis::Z, false) => Direction::North,
        }
    }

    /// `Direction.getNearest(x, y, z)`: the direction whose normal has the
    /// largest dot product, the first in `ALL` on a tie.
    pub fn nearest(d: Pos) -> Direction {
        let mut best = Direction::North;
        let mut best_dot = f32::MIN;
        for dir in Direction::ALL {
            let n = dir.normal();
            let dot = (d[0] * n[0] + d[1] * n[1] + d[2] * n[2]) as f32;
            if dot > best_dot {
                best_dot = dot;
                best = dir;
            }
        }
        best
    }

    pub fn name(self) -> &'static str {
        match self {
            Direction::Down => "down",
            Direction::Up => "up",
            Direction::North => "north",
            Direction::South => "south",
            Direction::West => "west",
            Direction::East => "east",
        }
    }

    pub fn parse(s: &str) -> Option<Direction> {
        Direction::ALL.into_iter().find(|d| d.name() == s)
    }
}
