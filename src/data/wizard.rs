use super::{spells::create_spells, Ticable};
use crate::config::Player;
use crate::data::arena::ArenaError;
use crate::data::spells::Spell;
use crate::data::stats::{Frame, WizardStats};
use crate::gfx::buffer::Buffer;
use crate::gfx::color::Color;
use crate::net::NetworkError;
use rand::{thread_rng, Rng};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::{error, fmt};

static CHARACTERS: &[u8; 256] = include_bytes!("../gfx/bin/wizards/characters.bin");

static STARTING_POSITIONS: &[&[(u8, u8)]; 7] = &[
    &[(1, 4), (13, 4)],
    &[(7, 1), (1, 8), (13, 8)],
    &[(1, 1), (13, 1), (1, 8), (13, 8)],
    &[(7, 0), (0, 3), (14, 3), (3, 9), (11, 9)],
    &[(7, 0), (0, 1), (14, 1), (0, 8), (7, 9), (14, 8)],
    &[(7, 0), (1, 1), (13, 1), (0, 6), (14, 6), (4, 9), (10, 9)],
    &[(0, 0), (7, 0), (14, 0), (0, 4), (14, 4), (0, 9), (7, 9), (14, 9)],
];

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum WizardCharacter {
    Jevarell,
    LargeFart,
    GreatFogey,
    Dyerarti,
    Gowin,
    Merlin,
    IlianRane,
    AsimonoZark,
}

impl WizardCharacter {
    pub fn as_bytes(&self) -> &[u8] {
        match self {
            WizardCharacter::Jevarell => &CHARACTERS[0..32],
            WizardCharacter::LargeFart => &CHARACTERS[32..64],
            WizardCharacter::GreatFogey => &CHARACTERS[64..96],
            WizardCharacter::Dyerarti => &CHARACTERS[96..128],
            WizardCharacter::Gowin => &CHARACTERS[128..160],
            WizardCharacter::Merlin => &CHARACTERS[160..192],
            WizardCharacter::IlianRane => &CHARACTERS[192..224],
            WizardCharacter::AsimonoZark => &CHARACTERS[224..256],
        }
    }

    pub fn as_buffer(&self, color: WizardColor) -> Buffer {
        Buffer::from_shorts(self.as_bytes(), color.into(), None)
    }
}

impl TryFrom<isize> for WizardCharacter {
    type Error = WizardError;

    fn try_from(value: isize) -> Result<Self, Self::Error> {
        use WizardCharacter::*;
        match value {
            0 => Ok(Jevarell),
            1 => Ok(LargeFart),
            2 => Ok(GreatFogey),
            3 => Ok(Dyerarti),
            4 => Ok(Gowin),
            5 => Ok(Merlin),
            6 => Ok(IlianRane),
            7 => Ok(AsimonoZark),
            _ => Err(WizardError::InvalidWizardCharacterValue),
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum WizardColor {
    BrightRed,
    BrightMagenta,
    BrightGreen,
    BrightCyan,
    Yellow,
    BrightYellow,
    White,
    BrightWhite,
}

impl TryFrom<isize> for WizardColor {
    type Error = WizardError;

    fn try_from(value: isize) -> Result<WizardColor, Self::Error> {
        use WizardColor::*;
        match value {
            0 => Ok(BrightRed),
            1 => Ok(BrightMagenta),
            2 => Ok(BrightGreen),
            3 => Ok(BrightCyan),
            4 => Ok(Yellow),
            5 => Ok(BrightYellow),
            6 => Ok(White),
            7 => Ok(BrightWhite),
            _ => Err(WizardError::InvalidWizardColorValue),
        }
    }
}

impl From<WizardColor> for Color {
    fn from(value: WizardColor) -> Color {
        match value {
            WizardColor::BrightRed => Color::BrightRed,
            WizardColor::BrightMagenta => Color::BrightMagenta,
            WizardColor::BrightGreen => Color::BrightGreen,
            WizardColor::BrightCyan => Color::BrightCyan,
            WizardColor::Yellow => Color::Yellow,
            WizardColor::BrightYellow => Color::BrightYellow,
            WizardColor::White => Color::White,
            WizardColor::BrightWhite => Color::BrightWhite,
        }
    }
}

#[derive(Debug)]
pub enum WizardError {
    InvalidWizardCharacterValue,
    InvalidWizardColorValue,
}

impl fmt::Display for WizardError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use WizardError::*;
        match self {
            InvalidWizardCharacterValue => write!(f, "Invalid wizard character value"),
            InvalidWizardColorValue => write!(f, "Invalid wizard color value"),
        }
    }
}

impl error::Error for WizardError {}

#[derive(Clone)]
pub struct LobbyWizard {
    pub player: Player,
    pub id: u32,
    pub ready: bool,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Wizard {
    pub player: Player,
    pub id: u32,
    pub alive: bool,
    pub disconnected: bool,
    pub spells: Vec<Spell>,
    pub stats: WizardStats,
}

pub struct ServerWizards {
    wizards: Vec<Wizard>,
}

impl ServerWizards {
    pub fn get(&self, id: u32) -> Result<&Wizard, NetworkError> {
        self.wizards.iter().find(|w| w.id == id).ok_or(NetworkError::GenericError)
    }

    pub fn get_mut(&mut self, id: u32) -> Result<&mut Wizard, NetworkError> {
        self.wizards.iter_mut().find(|w| w.id == id).ok_or(NetworkError::GenericError)
    }

    pub fn has_disconnected(&self, id: u32) -> Result<bool, NetworkError> {
        Ok(self.get(id)?.disconnected)
    }

    pub fn starting_positions(&self) -> Result<impl Iterator<Item = (u8, u8, &Wizard)>, ArenaError> {
        Ok(STARTING_POSITIONS
            .get(self.wizards.len() - 2)
            .ok_or(ArenaError::InvalidNumPlayers)?
            .iter()
            .zip(self.wizards.iter())
            .map(|((x, y), wiz)| (*x, *y, wiz)))
    }

    pub fn all_active_ids(&self) -> Vec<u32> {
        self.wizards
            .iter()
            .filter(|w| w.alive && !w.disconnected)
            .map(|w| w.id)
            .collect()
    }

    pub fn is_alive(&self, id: u32) -> Result<bool, NetworkError> {
        Ok(self.get(id)?.alive)
    }

    pub fn iter(&self) -> impl Iterator<Item = &Wizard> {
        self.wizards.iter()
    }

    pub fn check_for_winning_condition(&self) -> bool {
        self.wizards.iter().filter(|w| w.alive && !w.disconnected).count() == 1
    }

    pub fn winners(self) -> Vec<Player> {
        self.wizards
            .into_iter()
            .filter_map(|wizard| {
                if wizard.alive && !wizard.disconnected {
                    Some(wizard.player)
                } else {
                    None
                }
            })
            .collect()
    }

    pub fn len(&self) -> usize {
        self.wizards.len()
    }
}

impl From<LobbyWizards> for ServerWizards {
    fn from(wizards: LobbyWizards) -> Self {
        let mut wizards = wizards.players.into_values().map(Wizard::from).collect::<Vec<_>>();
        wizards.sort_by(|a, b| a.id.cmp(&b.id));
        Self { wizards }
    }
}

impl From<LobbyWizard> for Wizard {
    fn from(wizard: LobbyWizard) -> Self {
        let level = 0;
        let stats = WizardStats::new(&wizard, level);
        let spells = create_spells(stats.number_of_spells);
        Self {
            player: wizard.player,
            id: wizard.id,
            alive: true,
            disconnected: false,
            spells,
            stats,
        }
    }
}

#[derive(Default)]
pub struct LobbyWizards {
    pub players: HashMap<u32, LobbyWizard>,
}

impl LobbyWizards {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn join(&mut self, id: u32, player: Player) -> bool {
        if self.players.len() >= 8 {
            return false;
        }
        self.players.insert(
            id,
            LobbyWizard {
                player,
                id,
                ready: false,
            },
        );
        true
    }

    pub fn leave(&mut self, id: u32) -> Option<LobbyWizard> {
        self.players.remove(&id)
    }

    pub fn ready(&mut self, id: u32, ready: bool) -> bool {
        if let Some(player) = self.players.get_mut(&id) {
            player.ready = ready;
            true
        } else {
            false
        }
    }

    pub fn players(&self) -> impl Iterator<Item = LobbyWizard> {
        let mut vec: Vec<LobbyWizard> = self.players.values().cloned().collect();
        vec.sort_by(|a, b| a.id.cmp(&b.id));
        vec.into_iter()
    }

    pub fn is_ready(&self) -> bool {
        self.players.len() >= 2 && self.players.values().all(|w| w.ready)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GameWizard {
    pub id: u32,
    pub name: String,
    pub moves_left: u8,
    pub stats: WizardStats,
    frame_count: u8,
    current_frame: u8,
    pub buffers: [Buffer; 4],
}

impl GameWizard {
    pub fn update_stats(&mut self, stats: WizardStats) {
        self.buffers = stats.gfx.as_buffers();
        self.stats = stats;
    }

    pub fn is_engaged(&self, manoeuvre: u8) -> bool {
        let mut rng = thread_rng();
        self.stats.base.manoeuvre + rng.gen_range(0..=9) <= manoeuvre + rng.gen_range(0..=9)
    }

    pub fn defend_against_attack(&self, combat: u8) -> bool {
        let mut rng = thread_rng();
        combat + rng.gen_range(0..=9) >= self.stats.get_defence() + rng.gen_range(0..=9)
    }

    pub fn current_bytes(&self) -> [u8; 32] {
        self.stats
            .gfx
            .frames
            .get(self.current_frame as usize)
            .expect("Invalid Frame")
            .bytes
    }

    pub fn current_frame(&self) -> &Frame {
        self.stats.gfx.frames.get(self.current_frame as usize).expect("Invalid Frame")
    }

    pub fn defend_against_magical_attack(&self, spell_ability: u8) -> bool {
        let mut rng = thread_rng();
        spell_ability + rng.gen_range(0..=9) >= self.stats.base.magical_resistance + rng.gen_range(0..=9)
    }
}

impl From<&Wizard> for GameWizard {
    fn from(wizard: &Wizard) -> Self {
        GameWizard {
            id: wizard.id,
            name: wizard.player.name.clone(),
            moves_left: 0,
            stats: wizard.stats.clone(),
            frame_count: 0,
            current_frame: 0,
            buffers: wizard.stats.gfx.as_buffers(),
        }
    }
}

impl Ticable for GameWizard {
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
        if self.stats.shadow_form && self.current_frame % 2 == 0 {
            return None;
        }
        Some(self.buffers.get(self.current_frame as usize).unwrap())
    }

    fn current_tic(&self) -> &Buffer {
        self.buffers.get(self.current_frame as usize).unwrap()
    }
}
