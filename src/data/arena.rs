use crate::data::creation::GameCreation;
use crate::data::stats::CreationStats;
use crate::data::wizard::GameWizard;
use crate::data::Ticable;
use crate::gfx::buffer::{Buffer, MouseCursor};
use crate::gfx::color::Color;
use serde::{Deserialize, Serialize};
use std::{error, fmt};

use super::stats::Frame;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Spawn {
    Blob(GameCreation),
    Fire(GameCreation),
}

impl Ticable for Spawn {
    fn tic(&mut self) -> Option<&Buffer> {
        match self {
            Spawn::Blob(creation) => creation.tic(),
            Spawn::Fire(creation) => creation.tic(),
        }
    }

    fn current_tic(&self) -> &Buffer {
        match self {
            Spawn::Blob(creation) => creation.current_tic(),
            Spawn::Fire(creation) => creation.current_tic(),
        }
    }
}

#[derive(Clone, Default)]
pub struct Tile {
    pub spawn: Option<Spawn>,
    pub corpse: Option<GameCreation>,
    pub creation: Option<GameCreation>,
    pub wizard: Option<GameWizard>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TileSelection {
    pub x: u8,
    pub y: u8,
    pub cursor: MouseCursor,
    pub color: Color,
    pub valid: bool,
}

#[derive(Debug)]
pub enum ArenaError {
    InvalidNumPlayers,
}

impl fmt::Display for ArenaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use ArenaError::*;
        match self {
            InvalidNumPlayers => write!(f, "Invalid number of players"),
        }
    }
}

impl error::Error for ArenaError {}

#[derive(Clone)]
pub struct Arena {
    pub alignment: i8,
    pub tiles: Vec<Tile>,
    pub width: u8,
    pub height: u8,
}

impl Arena {
    pub fn new() -> Self {
        let width = 15;
        let height = 10;
        Self {
            alignment: 0,
            tiles: vec![Tile::default(); width as usize * height as usize],
            width,
            height,
        }
    }

    pub fn adjust_alignment(&mut self, alignment: i8) {
        self.alignment = self.alignment.saturating_add(alignment);
    }

    pub fn get_mut(&mut self, x: u8, y: u8) -> &mut Tile {
        self.tiles.get_mut(((y * self.width) + x) as usize).expect("tile")
    }

    pub fn get_blob(&self, x: u8, y: u8) -> &GameCreation {
        match self.get(x, y).spawn.as_ref().expect("spawn") {
            Spawn::Blob(blob) => blob,
            _ => panic!("blob"),
        }
    }

    pub fn get_creation(&self, x: u8, y: u8) -> &GameCreation {
        self.get(x, y).creation.as_ref().expect("creation")
    }

    pub fn get_mut_creation(&mut self, x: u8, y: u8) -> &mut GameCreation {
        self.get_mut(x, y).creation.as_mut().expect("creation")
    }

    pub fn get_wizard(&self, x: u8, y: u8) -> &GameWizard {
        self.get(x, y).wizard.as_ref().expect("wizard")
    }

    pub fn get_mut_wizard(&mut self, x: u8, y: u8) -> &mut GameWizard {
        self.get_mut(x, y).wizard.as_mut().expect("wizard")
    }

    pub fn get_corpse(&self, x: u8, y: u8) -> &GameCreation {
        self.get(x, y).corpse.as_ref().expect("corpse")
    }

    pub fn get(&self, x: u8, y: u8) -> &Tile {
        self.tiles.get(((y * self.width) + x) as usize).expect("tile")
    }

    fn each_tile_in_spell_range(&self, x: u8, y: u8, range: u8) -> impl Iterator<Item = (u8, u8, &Tile)> + '_ {
        self.each_tile().filter(move |(tile_x, tile_y, _)| {
            let distance = (*tile_x as isize - x as isize).pow(2) + (*tile_y as isize - y as isize).pow(2);
            distance <= (range as isize) && (distance > 0)
        })
    }

    fn each_tile_in_combat_range(&self, x: u8, y: u8, range: u8) -> impl Iterator<Item = (u8, u8, &Tile)> + '_ {
        self.each_tile().filter(move |(tile_x, tile_y, _)| {
            let distance = (*tile_x as isize - x as isize).pow(2) + (*tile_y as isize - y as isize).pow(2);
            distance <= (range as isize).pow(2) && (distance > 0)
        })
    }

    fn each_tile_in_flying_range(&self, x: u8, y: u8, movement: u8) -> impl Iterator<Item = (u8, u8, &Tile)> + '_ {
        self.each_tile().filter(move |(tile_x, tile_y, _)| {
            let distance = (*tile_x as isize - x as isize).pow(2) + (*tile_y as isize - y as isize).pow(2) - 1;
            distance <= (movement as isize).pow(2) && (distance >= 0)
        })
    }

    fn each_tile(&self) -> impl Iterator<Item = (u8, u8, &Tile)> {
        self.tiles
            .iter()
            .enumerate()
            .map(|(i, tile)| ((i % self.width as usize) as u8, (i / self.width as usize) as u8, tile))
    }

    fn all_attackable_opposition(&self, x: u8, y: u8, range: u8, id: u32) -> impl Iterator<Item = (u8, u8, &Tile)> {
        self.each_tile_in_spell_range(x, y, range).filter(move |(_, _, tile)| {
            match tile.spawn {
                Some(Spawn::Blob(ref blob)) => return blob.id != id,
                Some(Spawn::Fire(_)) => return false,
                _ => {}
            }
            if let Some(ref wizard) = tile.wizard {
                return wizard.id != id;
            }
            if let Some(ref creation) = tile.creation {
                return creation.id != id && creation.stats.attackable;
            }
            false
        })
    }

    pub fn all_subvertable_opposition_tiles(&self, x: u8, y: u8, range: u8, id: u32) -> Vec<(u8, u8)> {
        self.all_attackable_opposition(x, y, range, id)
            .filter_map(|(x, y, tile)| {
                if tile.spawn.is_some() {
                    return None;
                }
                if let Some(ref creation) = tile.creation {
                    if creation.stats.subvertable && tile.wizard.is_none() {
                        return Some((x, y));
                    }
                }
                None
            })
            .collect()
    }

    pub fn all_spawn_tiles(&mut self) -> Vec<(u8, u8)> {
        self.each_tile()
            .filter_map(|(x, y, tile)| if tile.spawn.as_ref().is_some() { Some((x, y)) } else { None })
            .collect()
    }

    pub fn spawn_fire(&mut self, x: u8, y: u8, fire: GameCreation) {
        let tile = self.get_mut(x, y);
        tile.corpse = None;
        tile.spawn = Some(Spawn::Fire(fire));
    }

    pub fn spawn_blob(&mut self, x: u8, y: u8, blob: GameCreation) {
        let tile = self.get_mut(x, y);
        tile.spawn = Some(Spawn::Blob(blob));
    }

    pub fn remove_spawn(&mut self, x: u8, y: u8) {
        let tile = self.get_mut(x, y);
        tile.spawn = None;
    }

    fn all_empty(&self, x: u8, y: u8, range: u8) -> impl Iterator<Item = (u8, u8, &Tile)> {
        self.each_tile_in_spell_range(x, y, range)
            .filter(|(_, _, tile)| tile.spawn.is_none() && tile.wizard.is_none() && tile.creation.is_none())
    }

    fn each_tile_mut(&mut self) -> impl Iterator<Item = (u8, u8, &mut Tile)> {
        self.tiles
            .iter_mut()
            .enumerate()
            .map(|(i, tile)| ((i % self.width as usize) as u8, (i / self.width as usize) as u8, tile))
    }

    pub fn find_wizard(&mut self, id: u32) -> &GameWizard {
        self.tiles
            .iter()
            .filter_map(|tile| tile.wizard.as_ref())
            .find(|wizard| wizard.id == id)
            .expect("wizard")
    }

    pub fn find_wizard_pos(&self, id: u32) -> (u8, u8) {
        self.each_tile()
            .filter_map(|(x, y, tile)| tile.wizard.as_ref().map(|wizard| (x, y, wizard)))
            .find(|(_, _, wizard)| wizard.id == id)
            .map(|(x, y, _)| (x, y))
            .expect("wizard")
    }

    pub fn maybe_find_wizard_pos(&self, id: u32) -> Option<(u8, u8)> {
        self.each_tile()
            .filter_map(|(x, y, tile)| tile.wizard.as_ref().map(|wizard| (x, y, wizard)))
            .find(|(_, _, wizard)| wizard.id == id)
            .map(|(x, y, _)| (x, y))
    }

    pub fn number_of_wizards(&self) -> usize {
        self.tiles.iter().filter(|tile| tile.wizard.is_some()).count()
    }

    pub fn get_visible_buffer(&self, x: u8, y: u8) -> &Buffer {
        if let Some(ref spawn) = self.get(x, y).spawn {
            match spawn {
                Spawn::Blob(blob) => return blob.current_tic(),
                Spawn::Fire(fire) => return fire.current_tic(),
            }
        }
        if let Some(ref creation) = self.get(x, y).creation {
            return creation.current_tic();
        }
        if let Some(ref wizard) = self.get(x, y).wizard {
            return wizard.current_tic();
        }
        panic!("visible piece")
    }

    pub fn get_visible_frame(&self, x: u8, y: u8) -> &Frame {
        if let Some(ref spawn) = self.get(x, y).spawn {
            match spawn {
                Spawn::Blob(blob) => return blob.current_frame(),
                Spawn::Fire(fire) => return fire.current_frame(),
            }
        }
        if let Some(ref creation) = self.get(x, y).creation {
            return creation.current_frame();
        }
        if let Some(ref wizard) = self.get(x, y).wizard {
            return wizard.current_frame();
        }
        panic!("visible piece")
    }

    pub fn find_wizard_mut(&mut self, id: u32) -> &mut GameWizard {
        self.tiles
            .iter_mut()
            .filter_map(|tile| tile.wizard.as_mut())
            .find(|wizard| wizard.id == id)
            .expect("wizard")
    }

    pub fn creation_spell_tiles(&self, x: u8, y: u8, range: u8) -> Vec<(u8, u8)> {
        self.all_empty(x, y, range).map(|(x, y, _)| (x, y)).collect()
    }

    pub fn cast_spell_on_attackable_tiles(&self, x: u8, y: u8, range: u8, id: u32) -> Vec<(u8, u8)> {
        self.all_attackable_opposition(x, y, range, id)
            .map(|(x, y, _)| (x, y))
            .collect()
    }

    pub fn ranged_combat_tiles(&self, x: u8, y: u8, range: u8) -> Vec<(u8, u8)> {
        self.each_tile_in_combat_range(x, y, range).map(|(x, y, _)| (x, y)).collect()
    }

    pub fn reset_moves(&mut self, id: u32) {
        for (_, _, tile) in self.each_tile_mut() {
            let blob = tile.spawn.is_some();
            if let Some(wizard) = tile.wizard.as_mut() {
                if wizard.id == id {
                    wizard.moves_left = if blob { 0 } else { wizard.stats.get_movement() };
                }
            }
            if let Some(creation) = tile.creation.as_mut() {
                if creation.id == id {
                    creation.moves_left = if blob {
                        0
                    } else if creation.stats.shadow_wood {
                        1
                    } else {
                        creation.stats.base.movement
                    };
                }
            }
        }
    }

    pub fn tiles_with_moves_left(&self, id: u32) -> Vec<(u8, u8)> {
        self.each_tile()
            .filter_map(|(x, y, tile)| {
                if let Some(GameCreation {
                    id: creation_id,
                    moves_left,
                    stats: CreationStats { shelter: false, .. },
                    ..
                }) = tile.creation
                {
                    if creation_id == id && moves_left > 0 {
                        return Some((x, y));
                    }
                } else if let Some(ref wizard) = tile.wizard {
                    if wizard.id == id && wizard.moves_left > 0 {
                        return Some((x, y));
                    }
                }
                None
            })
            .collect()
    }

    pub fn move_wizard(&mut self, id: u32, x: u8, y: u8) {
        let (sx, sy) = self.find_wizard_pos(id);
        self.get_mut(x, y).wizard = self.get_mut(sx, sy).wizard.take();
    }

    pub fn move_creation(&mut self, sx: u8, sy: u8, dx: u8, dy: u8) {
        if self.get(sx, sy).wizard.is_some() {
            self.get_mut(dx, dy).wizard = self.get_mut(sx, sy).wizard.take();
        }
        self.get_mut(dx, dy).creation = self.get_mut(sx, sy).creation.take();
    }

    pub fn wizard_movement_tiles(&self, x: u8, y: u8, id: u32) -> Vec<(u8, u8)> {
        self.each_tile_in_spell_range(x, y, 3)
            .filter_map(|(x, y, tile)| self.allow_wizard_movement_with_attack(x, y, tile, id))
            .collect()
    }

    pub fn wizard_flying_tiles(&self, x: u8, y: u8, movement: u8, id: u32) -> Vec<(u8, u8)> {
        self.each_tile_in_flying_range(x, y, movement)
            .filter_map(|(x, y, tile)| self.allow_wizard_movement_with_attack(x, y, tile, id))
            .collect()
    }

    fn allow_movement_with_attack(&self, x: u8, y: u8, tile: &Tile, id: u32) -> Option<(u8, u8)> {
        if let Some(ref spawn) = tile.spawn {
            match spawn {
                Spawn::Blob(ref creation) if creation.id != id => Some((x, y)),
                _ => None,
            }
        } else if let Some(ref creation) = tile.creation {
            if creation.id != id && creation.stats.attackable
                || (creation.stats.magic_wood && tile.wizard.as_ref().is_some_and(|wizard| wizard.id != id))
            {
                Some((x, y))
            } else {
                None
            }
        } else if let Some(ref wizard) = tile.wizard {
            if wizard.id != id {
                Some((x, y))
            } else {
                None
            }
        } else {
            Some((x, y))
        }
    }

    fn allow_wizard_movement_with_attack(&self, x: u8, y: u8, tile: &Tile, id: u32) -> Option<(u8, u8)> {
        if let Some(ref spawn) = tile.spawn {
            match spawn {
                Spawn::Blob(ref creation) if creation.id != id => Some((x, y)),
                _ => None,
            }
        } else if let Some(ref creation) = tile.creation {
            if (creation.id != id && creation.stats.attackable)
                || (creation.id == id && (creation.stats.mount || creation.stats.shelter) || creation.stats.magic_wood)
            {
                Some((x, y))
            } else {
                None
            }
        } else if let Some(ref wizard) = tile.wizard {
            if wizard.id != id {
                Some((x, y))
            } else {
                None
            }
        } else {
            Some((x, y))
        }
    }

    fn allow_attack(&self, x: u8, y: u8, tile: &Tile, id: u32) -> Option<(u8, u8)> {
        if let Some(ref creation) = tile.creation {
            if creation.id != id && creation.stats.attackable
                || (creation.stats.magic_wood && tile.wizard.as_ref().is_some_and(|wizard| wizard.id != id))
            {
                Some((x, y))
            } else {
                None
            }
        } else if let Some(ref wizard) = tile.wizard {
            if wizard.id != id {
                Some((x, y))
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn creation_movement_tiles(&self, x: u8, y: u8, id: u32) -> Vec<(u8, u8)> {
        self.each_tile_in_spell_range(x, y, 3)
            .filter_map(|(x, y, tile)| self.allow_movement_with_attack(x, y, tile, id))
            .collect()
    }

    pub fn creation_combat_tiles(&self, x: u8, y: u8, id: u32) -> Vec<(u8, u8)> {
        self.each_tile_in_spell_range(x, y, 3)
            .filter_map(|(x, y, tile)| self.allow_attack(x, y, tile, id))
            .collect()
    }

    pub fn wizard_combat_tiles(&self, x: u8, y: u8, id: u32) -> Vec<(u8, u8)> {
        self.each_tile_in_spell_range(x, y, 3)
            .filter_map(|(x, y, tile)| self.allow_attack(x, y, tile, id))
            .collect()
    }

    pub fn creation_flying_tiles(&self, x: u8, y: u8, movement: u8, id: u32) -> Vec<(u8, u8)> {
        self.each_tile_in_flying_range(x, y, movement)
            .filter_map(|(x, y, tile)| self.allow_movement_with_attack(x, y, tile, id))
            .collect()
    }

    pub fn neighbouring_foes_iter(&self, x: u8, y: u8, id: u32) -> impl Iterator<Item = (u8, u8)> + '_ {
        self.each_tile_in_spell_range(x, y, 3).filter_map(move |(x, y, tile)| {
            if tile.spawn.is_some() {
                None
            } else if let Some(ref creation) = tile.creation {
                if creation.id != id && creation.stats.attackable
                    || (creation.stats.magic_wood && tile.wizard.as_ref().is_some_and(|wizard| wizard.id != id))
                {
                    Some((x, y))
                } else {
                    None
                }
            } else if let Some(ref wizard) = tile.wizard {
                if wizard.id != id {
                    Some((x, y))
                } else {
                    None
                }
            } else {
                None
            }
        })
    }

    pub fn has_neighbouring_foes(&self, x: u8, y: u8, id: u32) -> bool {
        self.neighbouring_foes_iter(x, y, id).any(|_| true)
    }

    pub fn neighbouring_foes(&self, x: u8, y: u8, id: u32) -> Vec<(u8, u8)> {
        self.neighbouring_foes_iter(x, y, id).collect()
    }

    pub fn kill_creation(&mut self, x: u8, y: u8, corpse: bool) {
        let tile = self.get_mut(x, y);
        let creation = tile.creation.take();
        if corpse {
            tile.corpse = creation
        }
    }

    pub fn kill_wizard_and_creations(&mut self, id: u32) {
        for (_, _, tile) in self.each_tile_mut() {
            if let Some(ref spawn) = tile.spawn {
                match spawn {
                    Spawn::Blob(blob) => {
                        if id == blob.id {
                            tile.spawn = None;
                        }
                    }
                    Spawn::Fire(fire) => {
                        if id == fire.id {
                            tile.spawn = None;
                        }
                    }
                }
            }
            if let Some(ref wizard) = tile.wizard {
                if wizard.id == id {
                    tile.wizard = None;
                }
            }
            if let Some(ref creation) = tile.creation {
                if creation.id == id {
                    tile.creation = None;
                }
            }
            if let Some(ref corpse) = tile.corpse {
                if corpse.id == id {
                    tile.corpse = None;
                }
            }
        }
    }

    pub fn line_coords(sx: u8, sy: u8, dx: u8, dy: u8) -> Vec<(usize, usize)> {
        let mut coords = Vec::new();
        let mut sx = sx as isize * 16 + 8;
        let mut sy = sy as isize * 16 + 8;
        let dx = dx as isize * 16 + 8;
        let dy = dy as isize * 16 + 8;
        let delta_x = (dx - sx).abs();
        let delta_y = (dy - sy).abs();
        let sign_x = if sx < dx { 1 } else { -1 };
        let sign_y = if sy < dy { 1 } else { -1 };
        let mut err = delta_x - delta_y;
        loop {
            coords.push((sx as usize, sy as usize));
            if sx == dx && sy == dy {
                return coords;
            }
            let e2 = 2 * err;
            if e2 > -delta_y {
                err -= delta_y;
                sx += sign_x;
            }
            if e2 < delta_x {
                err += delta_x;
                sy += sign_y;
            }
        }
    }

    pub fn get_topmost_creations_and_corpses_coords(&self, id: u32) -> Vec<(u8, u8)> {
        self.each_tile()
            .filter_map(|(x, y, tile)| {
                if let Some(ref spawn) = tile.spawn {
                    match spawn {
                        Spawn::Blob(blob) => {
                            if id == blob.id {
                                return Some((x, y));
                            }
                        }
                        Spawn::Fire(fire) => {
                            if id == fire.id {
                                return Some((x, y));
                            }
                        }
                    }
                }
                if let Some(ref creation) = tile.creation {
                    if id == creation.id {
                        return Some((x, y));
                    }
                } else if let Some(ref corpse) = tile.corpse {
                    if id == corpse.id {
                        return Some((x, y));
                    }
                }
                None
            })
            .collect()
    }

    pub fn get_info_bufs(&self, x: u8, y: u8) -> Vec<Buffer> {
        let mut buf = Vec::new();
        if let Some(ref spawn) = self.get(x, y).spawn {
            match spawn {
                Spawn::Blob(blob) => buf.push(Buffer::from(&blob.stats)),
                Spawn::Fire(fire) => buf.push(Buffer::from(&fire.stats)),
            }
        }
        if let Some(ref creation) = self.get(x, y).creation {
            buf.push(Buffer::from(&creation.stats));
        }
        if let Some(ref wizard) = self.get(x, y).wizard {
            buf.push(Buffer::from(&wizard.stats));
        }
        if let Some(ref corpse) = self.get(x, y).corpse {
            buf.push(Buffer::from(&corpse.stats));
        }
        buf
    }

    pub fn line_of_sight(&mut self, sx: u8, sy: u8, dx: u8, dy: u8) -> bool {
        let mut arena = self.clone();
        for (x, y, tile) in arena.each_tile_mut() {
            tile.corpse = None;
            if (x == sx && y == sy) || (x == dx && y == dy) {
                tile.spawn = None;
                tile.creation = None;
                tile.wizard = None;
            } else if let Some(ref creation) = tile.creation {
                if creation.stats.transparent {
                    tile.creation = None;
                }
            }
        }
        let buf = Buffer::from(&arena);
        let coords = Self::line_coords(sx, sy, dx, dy);
        for (x, y) in coords.into_iter().step_by(4) {
            let color = buf.get_pixel(x, y).expect("pixel");
            if color != Color::Black.into() {
                return false;
            }
        }
        true
    }

    pub fn visible_corpse_tiles(&self, x: u8, y: u8, range: u8) -> Vec<(u8, u8)> {
        self.each_tile_in_spell_range(x, y, range)
            .filter_map(|(x, y, tile)| {
                if tile.spawn.is_some() || tile.wizard.is_some() || tile.creation.is_some() {
                    None
                } else if tile.corpse.is_some() {
                    Some((x, y))
                } else {
                    None
                }
            })
            .collect()
    }

    pub fn subvert(&mut self, x: u8, y: u8, id: u32) {
        self.get_mut_creation(x, y).id = id;
    }

    pub fn raise_dead(&mut self, x: u8, y: u8, id: u32) {
        let tile = self.get_mut(x, y);
        let mut creation = tile.corpse.take().expect("corpse");
        creation.id = id;
        creation.stats.undead = true;
        tile.creation = Some(creation);
    }

    pub fn all_combustable_shelter_tiles(&self) -> Vec<(u8, u8)> {
        self.each_tile()
            .filter_map(|(x, y, tile)| {
                if let Some(ref creation) = tile.creation {
                    if creation.stats.shelter && !creation.stats.magic_wood {
                        Some((x, y))
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect()
    }

    pub fn wizards_in_trees(&self) -> Vec<(u8, u8)> {
        self.each_tile()
            .filter_map(|(x, y, tile)| {
                if let Some(GameCreation {
                    stats: CreationStats { magic_wood: true, .. },
                    ..
                }) = tile.creation
                {
                    if tile.wizard.is_some() {
                        Some((x, y))
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect()
    }

    pub fn is_next_to_shadow_wood(&self, x: u8, y: u8) -> bool {
        self.each_tile_in_spell_range(x, y, 3)
            .any(|(_, _, tile)| tile.creation.as_ref().is_some_and(|creation| creation.stats.shadow_wood))
    }

    pub fn destroy_all_wizard_creations(&mut self, id: u32) {
        for (_, _, tile) in self.each_tile_mut() {
            if let Some(ref spawn) = tile.spawn {
                match spawn {
                    Spawn::Blob(blob) => {
                        if id == blob.id {
                            tile.spawn = None;
                        }
                    }
                    Spawn::Fire(fire) => {
                        if id == fire.id {
                            tile.spawn = None;
                        }
                    }
                }
            }
            if let Some(ref creation) = tile.creation {
                if id == creation.id {
                    tile.creation = None;
                }
            }
            if let Some(ref corpse) = tile.corpse {
                if id == corpse.id {
                    tile.corpse = None;
                }
            }
        }
    }
}

impl From<&mut Arena> for Buffer {
    fn from(arena: &mut Arena) -> Self {
        let mut arena_buf = Buffer::new(30, 20);
        for (x, y, tile) in arena.each_tile_mut() {
            if let Some(ref mut spawn) = tile.spawn {
                if let Some(buf) = spawn.tic() {
                    arena_buf.draw_buffer(buf, x as usize * 2, y as usize * 2);
                }
            } else if let Some(ref mut creation) = tile.creation {
                if let Some(buf) = creation.tic() {
                    arena_buf.draw_buffer(buf, x as usize * 2, y as usize * 2);
                }
            } else if let Some(ref mut wizard) = tile.wizard {
                if let Some(buf) = wizard.tic() {
                    arena_buf.draw_buffer(buf, x as usize * 2, y as usize * 2);
                }
            } else if let Some(ref creation) = tile.corpse {
                if let Some(ref buf) = creation.corpse_buf {
                    arena_buf.draw_buffer(buf, x as usize * 2, y as usize * 2);
                }
            }
        }
        arena_buf
    }
}

impl From<&Arena> for Buffer {
    fn from(arena: &Arena) -> Self {
        let mut arena_buf = Buffer::new(30, 20);
        for (x, y, tile) in arena.each_tile() {
            if let Some(ref spawn) = tile.spawn {
                let buf = spawn.current_tic();
                arena_buf.draw_buffer(buf, x as usize * 2, y as usize * 2);
            } else if let Some(ref creation) = tile.creation {
                let buf = creation.current_tic();
                arena_buf.draw_buffer(buf, x as usize * 2, y as usize * 2);
            } else if let Some(ref wizard) = tile.wizard {
                let buf = wizard.current_tic();
                arena_buf.draw_buffer(buf, x as usize * 2, y as usize * 2);
            } else if let Some(ref creation) = tile.corpse {
                if let Some(ref buf) = creation.corpse_buf {
                    arena_buf.draw_buffer(buf, x as usize * 2, y as usize * 2);
                }
            }
        }
        arena_buf
    }
}
