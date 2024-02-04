pub mod arena;
pub mod creation;
mod spellbook;
pub mod spells;
pub mod stats;
pub mod wizard;

use crate::gfx::buffer::Buffer;

pub trait Ticable {
    fn tic(&mut self) -> Option<&Buffer>;
    fn current_tic(&self) -> &Buffer;
}
