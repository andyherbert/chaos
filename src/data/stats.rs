use crate::data::wizard::LobbyWizard;
use crate::gfx::buffer::Buffer;
use crate::gfx::color::Color;
use rand::{thread_rng, Rng};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Frame {
    pub bytes: [u8; 32],
    pub fg: Color,
    pub bg: Option<Color>,
}

impl Frame {
    pub fn swap_colors(&self) -> Self {
        let bg = self.bg.unwrap_or(Color::Black);
        Frame {
            bytes: self.bytes,
            fg: bg,
            bg: Some(self.fg),
        }
    }
}

impl From<&Frame> for Buffer {
    fn from(frame: &Frame) -> Self {
        Buffer::from_shorts(&frame.bytes, frame.fg, frame.bg)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Gfx {
    pub timing: u8,
    pub frames: [Frame; 4],
    pub corpse: Option<Frame>,
}

impl Gfx {
    pub fn change_frame_bytes(&mut self, bytes: &[&[u8; 32]; 4]) {
        for (index, frame) in self.frames.iter_mut().enumerate() {
            frame.bytes.copy_from_slice(bytes[index]);
        }
    }

    pub fn as_buffers(&self) -> [Buffer; 4] {
        [
            Buffer::from(&self.frames[0]),
            Buffer::from(&self.frames[1]),
            Buffer::from(&self.frames[2]),
            Buffer::from(&self.frames[3]),
        ]
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BaseStats {
    pub name: String,
    pub combat: u8,
    pub ranged_combat: u8,
    pub range: u8,
    pub defence: u8,
    pub movement: u8,
    pub manoeuvre: u8,
    pub magical_resistance: u8,
}

impl From<&BaseStats> for Buffer {
    fn from(stats: &BaseStats) -> Self {
        use Color::*;
        let mut buf = Buffer::new(32, 24);
        buf.draw_text(&stats.name, 4, 2, BrightYellow);
        buf.draw_text("COMBAT=", 4, 6, BrightCyan);
        buf.draw_text(&stats.combat.to_string(), 11, 6, BrightWhite);
        buf.draw_text("RANGED COMBAT=", 4, 8, BrightCyan);
        buf.draw_text(&stats.ranged_combat.to_string(), 18, 8, BrightWhite);
        buf.draw_text("RANGE=", 20, 8, BrightCyan);
        buf.draw_text(&stats.range.to_string(), 26, 8, BrightWhite);
        buf.draw_text("DEFENCE=", 4, 10, BrightCyan);
        buf.draw_text(&stats.defence.to_string(), 12, 10, BrightWhite);
        buf.draw_text("MOVEMENT ALLOWANCE=", 4, 12, BrightCyan);
        buf.draw_text(&stats.movement.to_string(), 23, 12, BrightWhite);
        buf.draw_text("MANOEUVRE RATING=", 4, 14, BrightCyan);
        buf.draw_text(&stats.manoeuvre.to_string(), 21, 14, BrightWhite);
        buf.draw_text("MAGICAL RESISTANCE=", 4, 16, BrightCyan);
        buf.draw_text(&stats.magical_resistance.to_string(), 23, 16, BrightWhite);
        buf
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CreationStats {
    pub base: BaseStats,
    pub casting_chance: u8,
    pub alignment: i8,
    pub mount: bool,
    pub flying: bool,
    pub undead: bool,
    pub transparent: bool,
    pub subvertable: bool,
    pub attackable: bool,
    pub dragon: bool,
    pub shelter: bool,
    pub magic_wood: bool,
    pub shadow_wood: bool,
    pub gfx: Gfx,
}

impl From<&CreationStats> for Buffer {
    fn from(stats: &CreationStats) -> Self {
        use Color::*;
        let mut buf = Buffer::from(&stats.base);
        match stats.alignment.cmp(&0) {
            Ordering::Less => {
                let text = format!("(CHAOS {})", stats.alignment.abs());
                buf.draw_text(&text, stats.base.name.len() + 5, 2, BrightMagenta);
            }
            Ordering::Greater => {
                let text = format!("(LAW {})", stats.alignment);
                buf.draw_text(&text, stats.base.name.len() + 5, 2, BrightCyan);
            }
            _ => {}
        }
        let mut properties = Vec::new();
        if stats.mount {
            properties.push("MOUNT");
        }
        if stats.flying {
            properties.push("FLYING");
        }
        if stats.undead {
            properties.push("UNDEAD");
        }
        let text = properties.join(",");
        buf.draw_text(&text, 4, 4, BrightGreen);
        buf.border(0, 0, 32, 24, BrightGreen, Black);
        buf
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AttackBuff {
    MagicKnife,
    MagicSword,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum DefenceBuff {
    MagicShield,
    MagicArmour,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WizardStats {
    pub base: BaseStats,
    pub number_of_spells: u8,
    pub spell_ability: u8,
    pub attack_buff: Option<AttackBuff>,
    pub defence_buff: Option<DefenceBuff>,
    pub magic_wings: bool,
    pub magic_bow: bool,
    pub shadow_form: bool,
    pub gfx: Gfx,
}

static MAGIC_KNIFE: &[u8] = include_bytes!("../gfx/bin/wizards/magic_knife.bin");
static MAGIC_SWORD: &[u8] = include_bytes!("../gfx/bin/wizards/magic_sword.bin");
static MAGIC_SHIELD: &[u8] = include_bytes!("../gfx/bin/wizards/magic_shield.bin");
static MAGIC_ARMOUR: &[u8] = include_bytes!("../gfx/bin/wizards/magic_armour.bin");
static MAGIC_WINGS: &[u8] = include_bytes!("../gfx/bin/wizards/magic_wings.bin");
static MAGIC_BOW: &[u8] = include_bytes!("../gfx/bin/wizards/magic_bow.bin");

impl WizardStats {
    pub fn new(wizard: &LobbyWizard, level: u8) -> Self {
        let mut rng = thread_rng();
        let combat = 1 + (rng.gen_range(0..=9) / 2) + (level / 2);
        let defence = 1 + (rng.gen_range(0..=9) / 2) + (level / 2);
        let manoeuvre = 3 + (rng.gen_range(0..=9) / 2) + (level / 4);
        let magical_resistance = 6 + (rng.gen_range(0..=9) / 4);
        let number_of_spells = (11 + (rng.gen_range(0..=9) / 4) + level).min(20);
        let r = rng.gen_range(0..=9);
        let spell_ability = if r >= (5 - (level / 2)) { r / 4 } else { 0 };
        let frame = Frame::from(&wizard.player);
        let gfx = Gfx {
            timing: 30,
            frames: [frame.clone(), frame.clone(), frame.clone(), frame],
            corpse: None,
        };
        WizardStats {
            base: BaseStats {
                name: wizard.player.name.clone(),
                combat,
                defence,
                movement: 1,
                manoeuvre,
                magical_resistance,
                ranged_combat: 0,
                range: 0,
            },
            attack_buff: None,
            defence_buff: None,
            magic_wings: false,
            magic_bow: false,
            number_of_spells,
            spell_ability,
            shadow_form: false,
            gfx,
        }
    }

    pub fn magic_knife(&mut self) {
        self.gfx.change_frame_bytes(&[
            &MAGIC_KNIFE[0..32].try_into().expect("Invalid frame"),
            &MAGIC_KNIFE[32..64].try_into().expect("Invalid frame"),
            &MAGIC_KNIFE[64..96].try_into().expect("Invalid frame"),
            &MAGIC_KNIFE[96..].try_into().expect("Invalid frame"),
        ]);
    }

    pub fn magic_sword(&mut self) {
        self.gfx.change_frame_bytes(&[
            &MAGIC_SWORD[0..32].try_into().expect("Invalid frame"),
            &MAGIC_SWORD[32..64].try_into().expect("Invalid frame"),
            &MAGIC_SWORD[64..96].try_into().expect("Invalid frame"),
            &MAGIC_SWORD[96..].try_into().expect("Invalid frame"),
        ]);
    }

    pub fn magic_shield(&mut self) {
        let bytes = &MAGIC_SHIELD[..].try_into().expect("Invalid frame");
        self.gfx.change_frame_bytes(&[bytes, bytes, bytes, bytes]);
    }

    pub fn magic_armour(&mut self) {
        let bytes = &MAGIC_ARMOUR[..].try_into().expect("Invalid frame");
        self.gfx.change_frame_bytes(&[bytes, bytes, bytes, bytes]);
    }

    pub fn magic_wings(&mut self) {
        self.magic_wings = true;
        self.gfx.change_frame_bytes(&[
            &MAGIC_WINGS[0..32].try_into().expect("Invalid frame"),
            &MAGIC_WINGS[32..64].try_into().expect("Invalid frame"),
            &MAGIC_WINGS[64..96].try_into().expect("Invalid frame"),
            &MAGIC_WINGS[96..].try_into().expect("Invalid frame"),
        ]);
    }

    pub fn magic_bow(&mut self) {
        self.magic_bow = true;
        self.gfx.change_frame_bytes(&[
            &MAGIC_BOW[0..32].try_into().expect("Invalid frame"),
            &MAGIC_BOW[32..64].try_into().expect("Invalid frame"),
            &MAGIC_BOW[64..96].try_into().expect("Invalid frame"),
            &MAGIC_BOW[96..].try_into().expect("Invalid frame"),
        ]);
    }

    pub fn get_combat(&self) -> u8 {
        let mut combat = self.base.combat;
        if let Some(ref buff) = self.attack_buff {
            match buff {
                AttackBuff::MagicKnife => {
                    combat += 2;
                }
                AttackBuff::MagicSword => {
                    combat += 4;
                }
            }
        }
        combat.min(9)
    }

    pub fn get_defence(&self) -> u8 {
        let mut defence = self.base.defence;
        if let Some(ref buff) = self.defence_buff {
            match buff {
                DefenceBuff::MagicShield => {
                    defence += 2;
                }
                DefenceBuff::MagicArmour => {
                    defence += 4;
                }
            }
        }
        if self.shadow_form {
            defence += 3;
        }
        defence.min(9)
    }

    pub fn get_ranged_combat(&self) -> u8 {
        if self.magic_bow {
            3
        } else {
            0
        }
    }

    pub fn get_range(&self) -> u8 {
        if self.magic_bow {
            6
        } else {
            0
        }
    }

    pub fn get_movement(&self) -> u8 {
        if self.shadow_form {
            self.base.movement + 2
        } else {
            self.base.movement
        }
    }
}

impl From<&WizardStats> for Buffer {
    fn from(stats: &WizardStats) -> Self {
        use Color::*;
        let mut buf = Buffer::new(32, 24);
        let base_buf = Buffer::from(&stats.base);
        buf.draw_buffer(&base_buf, 0, 0);
        let mut properties = Vec::new();
        if let Some(ref buff) = stats.attack_buff {
            match buff {
                AttackBuff::MagicKnife => {
                    properties.push("KNIFE");
                }
                AttackBuff::MagicSword => {
                    properties.push("SWORD");
                }
            }
        }
        if let Some(ref buff) = stats.defence_buff {
            match buff {
                DefenceBuff::MagicShield => {
                    properties.push("SHIELD");
                }
                DefenceBuff::MagicArmour => {
                    properties.push("ARMOUR");
                }
            }
        }
        if stats.magic_wings {
            properties.push("FLYING");
        }
        buf.draw_text(&properties.join(","), 4, 4, BrightGreen);
        let text = format!("SPELLS={}  ABILITY={}", stats.number_of_spells, stats.spell_ability);
        buf.draw_text(&text, 4, 18, BrightYellow);
        buf.draw_text(&stats.get_combat().to_string(), 11, 6, BrightWhite);
        buf.draw_text(&stats.get_defence().to_string(), 12, 10, BrightWhite);
        buf.draw_text(&stats.get_ranged_combat().to_string(), 18, 8, BrightWhite);
        buf.draw_text(&stats.get_range().to_string(), 26, 8, BrightWhite);
        buf.border(0, 0, 32, 24, BrightGreen, Black);
        buf
    }
}
