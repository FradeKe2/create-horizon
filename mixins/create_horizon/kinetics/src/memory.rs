// A level held in memory: the tests' world, and a model of what the game's
// glue does (blocks, block entities, which positions were synced or broken).

use crate::blocks::Block;
use crate::engine::{Engine, Networks, World};
use crate::geom::Pos;
use crate::state::Kinetic;
use std::collections::{BTreeSet, HashMap};

#[derive(Default)]
pub struct MemoryWorld {
    pub blocks: HashMap<Pos, Block>,
    pub entities: HashMap<Pos, Kinetic>,
    pub synced: BTreeSet<Pos>,
    pub destroyed: Vec<Pos>,
    pub nets: Networks,
}

impl World for MemoryWorld {
    fn block(&self, pos: Pos) -> Option<Block> {
        self.blocks.get(&pos).copied()
    }
    fn loaded(&self, _pos: Pos) -> bool {
        true
    }
    fn kinetic(&self, pos: Pos) -> Option<Kinetic> {
        self.entities.get(&pos).cloned()
    }
    fn set_kinetic(&mut self, pos: Pos, k: Kinetic) {
        if self.blocks.contains_key(&pos) {
            self.entities.insert(pos, k);
        }
    }
    fn send_data(&mut self, pos: Pos) {
        self.synced.insert(pos);
    }
    fn destroy_block(&mut self, pos: Pos) {
        self.blocks.remove(&pos);
        self.entities.remove(&pos);
        self.destroyed.push(pos);
    }
}

impl MemoryWorld {
    /// Places a block with a fresh block entity, as the game does.
    pub fn place(&mut self, pos: Pos, block: Block) {
        self.blocks.insert(pos, block);
        self.entities.insert(pos, Kinetic::default());
    }

    /// A player breaks the block: the block entity's `remove`.
    pub fn break_block(&mut self, pos: Pos) {
        let (Some(b), Some(k)) = (self.blocks.remove(&pos), self.entities.remove(&pos)) else {
            return;
        };
        self.with(|e| e.removed(pos, &b, &k));
    }

    /// Runs `f` with an engine over this world and its networks.
    pub fn with<R>(&mut self, f: impl FnOnce(&mut Engine<'_, MemoryWorld>) -> R) -> R {
        let mut nets = std::mem::take(&mut self.nets);
        let r = {
            let mut e = Engine::new(self, &mut nets);
            f(&mut e)
        };
        self.nets = nets;
        r
    }

    /// One server tick: every block entity, in a fixed order.
    pub fn tick(&mut self) {
        let mut order: Vec<Pos> = self.entities.keys().copied().collect();
        order.sort();
        for pos in order {
            self.with(|e| e.tick(pos));
        }
    }

    pub fn ticks(&mut self, n: usize) {
        for _ in 0..n {
            self.tick();
        }
    }

    pub fn speed(&self, pos: Pos) -> f32 {
        self.entities.get(&pos).map_or(0.0, |k| k.speed())
    }
}
