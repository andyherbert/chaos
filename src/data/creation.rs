use super::Ticable;
use crate::data::stats::{CreationStats, Frame};
use crate::gfx::buffer::Buffer;
use crate::gfx::color::Color;
use rand::{thread_rng, Rng};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GameCreation {
    pub id: u32,
    pub moves_left: u8,
    pub stats: CreationStats,
    pub frame_count: u8,
    pub current_frame: u8,
    pub buffers: [Buffer; 4],
    pub corpse_buf: Option<Buffer>,
    pub illusion: bool,
}

impl GameCreation {
    pub fn new(id: u32, stats: CreationStats) -> Self {
        let buffers = stats.gfx.as_buffers();
        let corpse_buf = stats.gfx.corpse.as_ref().map(Buffer::from);
        Self {
            id,
            moves_left: 0,
            stats,
            frame_count: 0,
            current_frame: 0,
            buffers,
            corpse_buf,
            illusion: false,
        }
    }

    pub fn has_a_corpse(&self) -> bool {
        !(self.illusion || self.stats.undead || self.stats.magic_wood || self.stats.shadow_wood)
    }

    pub fn is_engaged(&self, manoeuvre: u8) -> bool {
        let mut rng = thread_rng();
        self.stats.base.manoeuvre + rng.gen_range(0..=9) <= manoeuvre + rng.gen_range(0..=9)
    }

    pub fn defend_against_attack(&self, combat: u8) -> bool {
        let mut rng = thread_rng();
        combat + rng.gen_range(0..=9) >= self.stats.base.defence + rng.gen_range(0..=9)
    }

    pub fn defend_against_magical_attack(&self, spell_ability: u8) -> bool {
        let mut rng = thread_rng();
        spell_ability + rng.gen_range(0..=9) >= self.stats.base.magical_resistance + rng.gen_range(0..=9)
    }

    pub fn current_bytes(&self) -> [u8; 32] {
        self.stats.gfx.frames.get(self.current_frame as usize).unwrap().bytes
    }

    pub fn current_frame(&self) -> &Frame {
        self.stats.gfx.frames.get(self.current_frame as usize).unwrap()
    }

    pub fn projectile_color(&self) -> Color {
        self.stats.gfx.frames.first().unwrap().fg
    }

    pub fn should_disappear(&self) -> bool {
        let mut rng = thread_rng();
        rng.gen_range(0..=9) >= 8
    }
}

impl Ticable for GameCreation {
    fn tic(&mut self) -> Option<&Buffer> {
        if self.frame_count == self.stats.gfx.timing {
            self.frame_count = 0;
            self.current_frame += 1;
            if self.current_frame >= 4 {
                self.current_frame = 0;
            }
        } else {
            self.frame_count += 1;
        }
        Some(self.buffers.get(self.current_frame as usize).unwrap())
    }

    fn current_tic(&self) -> &Buffer {
        self.buffers.get(self.current_frame as usize).unwrap()
    }
}
