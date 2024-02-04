use super::{
    spellbook::SPELLS,
    stats::{AttackBuff, DefenceBuff},
};
use crate::data::stats::CreationStats;
use crate::gfx::buffer::Buffer;
use crate::gfx::color::Color::*;
use rand::{seq::SliceRandom, thread_rng, Rng};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

impl Spell {
    pub fn random() -> Self {
        let mut rng = thread_rng();
        SPELLS[1..].choose(&mut rng).expect("spell").clone()
    }

    pub fn is_creation(&self) -> bool {
        matches!(self.kind, SpellKind::Creation(_))
    }

    fn cast_chance(&self, alignment: i8, spell_ability: u8) -> i8 {
        let mut chance = self.chance as i8;
        if (self.alignment > 0 && alignment > 0) || (self.alignment < 0 && alignment < 0) {
            chance += alignment.abs() / 4;
        }
        (chance + spell_ability as i8).min(9)
    }

    pub fn cast(&self, alignment: i8, spell_ability: u8) -> bool {
        let chance = self.cast_chance(alignment, spell_ability);
        let mut rng = thread_rng();
        rng.gen_range(0..=9) <= chance
    }

    pub fn as_info_buffer(&self, alignment: i8, spell_ability: u8) -> Buffer {
        let mut buf = Buffer::new(32, 24);
        let chance = self.cast_chance(alignment, spell_ability);
        if let SpellKind::Creation(ref stats) = self.kind {
            let stats_buf = Buffer::from(stats);
            buf.draw_buffer(&stats_buf, 0, 0);
            buf.border(0, 0, 32, 24, BrightGreen, Black);
            let perc = (chance + 1) * 10;
            let text = format!("CASTING CHANCE={}%", perc);
            buf.draw_text(&text, 4, 18, BrightCyan);
        } else {
            buf.border(0, 0, 32, 24, BrightBlue, BrightCyan);
            buf.draw_text(&self.name, 5, 6, BrightYellow);
            match self.alignment.cmp(&0) {
                Ordering::Less => {
                    let text = format!("(CHAOS {})", self.alignment.abs());
                    buf.draw_text(&text, 5, 8, BrightMagenta);
                }
                Ordering::Greater => {
                    let text = format!("(LAW {})", self.alignment);
                    buf.draw_text(&text, 5, 8, BrightCyan);
                }
                _ => {}
            }
            buf.draw_text("CASTING CHANCE=", 5, 12, BrightGreen);
            let text = format!("{}%", (chance + 1) * 10);
            buf.draw_text(&text, 20, 12, BrightYellow);
            buf.draw_text("RANGE=", 5, 16, BrightGreen);
            let range = self.range / 2;
            let text = if range > 10 { "20".to_string() } else { range.to_string() };
            buf.draw_text(&text, 11, 16, BrightYellow);
        }
        buf
    }

    pub fn as_name_buffer(&self, world_alignment: i8, spell_ability: u8) -> Buffer {
        let mut buf = Buffer::new(self.name.len() + 1, 2);
        let chance = self.cast_chance(world_alignment, spell_ability);
        let color = match chance {
            0..=1 => BrightMagenta,
            2..=3 => BrightGreen,
            4..=5 => BrightCyan,
            6..=7 => BrightYellow,
            8..=9 => BrightWhite,
            _ => unreachable!("Invalid chance value"),
        };
        match self.alignment.cmp(&0) {
            Ordering::Less => buf.draw_text("*", 0, 0, color),
            Ordering::Equal => buf.draw_text("-", 0, 0, color),
            Ordering::Greater => buf.draw_text("^", 0, 0, color),
        }
        buf.draw_text(&self.name, 1, 0, color);
        buf
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum SpellKind {
    Disbelieve,
    Creation(CreationStats),
    MagicFire(CreationStats),
    GooeyBlob(CreationStats),
    MagicWood(CreationStats),
    ShadowWood(CreationStats),
    Shelter(CreationStats),
    Wall(CreationStats),
    MagicBolt,
    Lightning,
    MagicalAttack(u8),
    WizardAttackBuff(AttackBuff),
    WizardDefenceBuff(DefenceBuff),
    MagicBow,
    MagicWings,
    WorldAlignment,
    ShadowForm,
    Subversion,
    RaiseDead,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Spell {
    pub name: String,
    pub chance: u8,
    pub range: u8,
    pub alignment: i8,
    pub kind: SpellKind,
}

pub fn create_spells(number_of_spells: u8) -> Vec<Spell> {
    let mut spells = vec![Spell {
        name: "DISBELIEVE".to_string(),
        chance: 9,
        range: u8::MAX,
        alignment: 0,
        kind: SpellKind::Disbelieve,
    }];
    let mut rng = thread_rng();
    for _ in 1..number_of_spells {
        let spell = SPELLS.choose(&mut rng).expect("spell").clone();
        spells.push(spell);
    }
    spells
}
