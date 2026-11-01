// Create's kinetics for Horizon.
//
// A Rust port of the rotation and stress system of Create
// (https://github.com/Creators-of-Create/Create), whose code is MIT:
// Copyright (c) The Create Team / The Creators of Create. See NOTICE.md.
// It has no dependency on Horizon: the mod's helpers give it a `World`.

pub mod blocks;
pub mod engine;
pub mod geom;
pub mod memory;
pub mod render;
pub mod state;

pub use blocks::{Block, Kind};
pub use engine::{Engine, Network, Networks, World};
pub use geom::{Axis, Direction, Pos};
pub use state::{Kinetic, Value};

#[cfg(test)]
mod tests;
