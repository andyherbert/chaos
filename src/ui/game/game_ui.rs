use super::client_state::ClientState;
use crate::config::Player;
use crate::data::arena::Arena;
use crate::data::spells::Spell;
use crate::data::stats::Frame;
use crate::data::wizard::WizardColor;
use crate::error::ChaosError;
use crate::gfx::buffer::{Buffer, MouseCursor};
use crate::gfx::color::Color::{self, *};
use crate::gfx::fx::{ATTACK_FX, DRAGON_BURN_FX, EXPLODING_CIRCLE_FX, EXPLOSION_FX, TWIRL_FX};
use crate::window::{Key, Window};
use std::cmp::Ordering;
use std::time::Instant;

fn preview_spell_coords(x: usize, y: usize, spells: &[Spell]) -> Option<usize> {
    if (2..22).contains(&y) && (1..31).contains(&x) {
        let index = ((y - 2) / 2 * 2) + (x - 1) / 16;
        if let Some(spell) = spells.get(index) {
            let len = spell.name.len();
            if index % 2 == 0 {
                if (x - 1) <= len {
                    return Some(index);
                }
            } else if (x - 17) <= len {
                return Some(index);
            }
        }
    }
    None
}

fn preview_arena_coords(x: usize, y: usize) -> Option<(u8, u8)> {
    if (33..63).contains(&x) && (1..21).contains(&y) {
        let (x, y) = ((x - 33) / 2, (y - 1) / 2);
        Some((x as u8, y as u8))
    } else {
        None
    }
}

fn name_coords(x: usize, y: usize) -> Option<usize> {
    if (74..86).contains(&x) && (4..20).contains(&y) {
        let index = (y - 4) / 2;
        Some(index)
    } else {
        None
    }
}

#[derive(Default, PartialEq)]
enum MousePosition {
    #[default]
    None,
    Spell(usize),
    Tile(u8, u8),
    Name(usize),
}

pub struct InfoPanel {
    buf: Buffer,
    pos: MousePosition,
    current_buf_index: usize,
    wizards: Vec<(u32, String)>,
}

impl InfoPanel {
    pub fn new() -> Self {
        Self {
            buf: Buffer::new(32, 24),
            pos: MousePosition::None,
            current_buf_index: 0,
            wizards: Vec::new(),
        }
    }

    pub fn add_wizard(&mut self, id: u32, name: &str) {
        self.wizards.push((id, name.to_string()));
    }

    pub fn draw_names(&mut self, win: &mut Window, state: &mut ClientState) -> Result<(), ChaosError> {
        self.buf.clear();
        let text = if state.turns_left == 1 {
            format!("{} TURN LEFT", state.turns_left)
        } else {
            format!("{} TURNS LEFT", state.turns_left)
        };
        self.buf.screen_border(&text, BrightGreen, Black);
        for (i, (id, name)) in self.wizards.iter().enumerate() {
            let mut name_x = (32 - name.len()) / 2;
            let name_y = 4 + i * 2;
            if let Some((x, y)) = state.arena.maybe_find_wizard_pos(*id) {
                self.buf.draw_text(name, name_x, name_y, BrightYellow);
                name_x += name.len();
                let buf = state.arena.get_visible_buffer(x, y);
                self.buf.draw_buffer(buf, name_x, name_y);
            } else {
                self.buf.draw_text(name, name_x, name_y, BrightRed);
            }
        }
        if let MousePosition::Name(index) = self.pos {
            if let Some((id, _)) = self.wizards.get(index) {
                if let Some((x, y)) = state.arena.maybe_find_wizard_pos(*id) {
                    let frame = state.arena.get_visible_frame(x, y).swap_colors();
                    let buf = Buffer::from(&frame);
                    win.buf.draw_buffer(&buf, x as usize * 2 + 33, y as usize * 2 + 1);
                    for (x, y) in state.arena.get_topmost_creations_and_corpses_coords(*id) {
                        let frame = state.arena.get_visible_frame(x, y).swap_colors();
                        let buf = Buffer::from(&frame);
                        win.buf.draw_buffer(&buf, x as usize * 2 + 33, y as usize * 2 + 1);
                    }
                }
            }
        }
        Ok(())
    }

    fn get_mouse_over(&mut self, win: &mut Window, state: &mut ClientState) -> MousePosition {
        if let Some((x, y)) = win.mouse_coords() {
            if let Some(index) = preview_spell_coords(x, y, &state.wizard.spells) {
                return MousePosition::Spell(index);
            } else if let Some((x, y)) = preview_arena_coords(x, y) {
                return MousePosition::Tile(x, y);
            } else if let Some(index) = name_coords(x, y) {
                return MousePosition::Name(index);
            }
        }
        MousePosition::None
    }

    pub fn render(&mut self, win: &mut Window, state: &mut ClientState) -> Result<(), ChaosError> {
        let now = self.get_mouse_over(win, state);
        if self.pos != now {
            match now {
                MousePosition::Spell(index) => {
                    if let Some(spell) = state.wizard.spells.get(index) {
                        let buf = spell.as_info_buffer(state.arena.alignment, state.wizard.stats.spell_ability);
                        self.buf.draw_buffer(&buf, 0, 0);
                    }
                }
                MousePosition::Tile(x, y) => {
                    let bufs = state.arena.get_info_bufs(x, y);
                    if !bufs.is_empty() {
                        let buf = bufs.first().expect("invalid index");
                        self.buf.draw_buffer(buf, 0, 0);
                    }
                }
                _ => {}
            }
            self.pos = now;
            self.current_buf_index = 0;
        } else if let MousePosition::Tile(x, y) = now {
            let bufs = state.arena.get_info_bufs(x, y);
            if !bufs.is_empty() {
                if self.current_buf_index >= bufs.len() {
                    self.current_buf_index = 0;
                }
                if win.is_down_pressed() && self.current_buf_index < bufs.len() - 1 {
                    self.current_buf_index += 1;
                }
                if win.is_up_pressed() && self.current_buf_index > 0 {
                    self.current_buf_index -= 1;
                }
                let buf = bufs.get(self.current_buf_index).expect("invalid index");
                self.buf.draw_buffer(buf, 0, 0);
                if bufs.len() > 1 {
                    let text = format!("PAGE {}/{} (UP/DOWN)", self.current_buf_index + 1, bufs.len());
                    self.buf.screen_border(&text, BrightGreen, Black);
                }
            } else {
                self.draw_names(win, state)?;
            }
        } else if let MousePosition::None = now {
            self.draw_names(win, state)?;
        } else if let MousePosition::Name(_) = now {
            self.draw_names(win, state)?;
        }
        win.buf.draw_buffer(&self.buf, 64, 0);
        Ok(())
    }
}

pub struct GameUI {
    pub panel: InfoPanel,
}

impl GameUI {
    pub fn new(win: &mut Window, state: &mut ClientState) -> Self {
        let ui = GameUI { panel: InfoPanel::new() };
        win.buf.clear();
        let text = format!("{}'S SPELLS", state.wizard.player.name);
        win.buf.draw_text(&text, 2, 0, BrightYellow);
        ui.update_alignment(win, state);
        ui.update_spells(win, state);
        ui.border(win, BrightBlue);
        ui
    }

    pub fn border(&self, win: &mut Window, color: Color) {
        win.buf.border(32, 0, 32, 22, color, BrightBlack);
    }

    pub fn wait_for(&mut self, win: &mut Window, state: &mut ClientState, ms: u128) -> Result<(), ChaosError> {
        let now = Instant::now();
        loop {
            if now.elapsed().as_millis() >= ms {
                return Ok(());
            }
            win.update()?;
            self.render(win, state)?;
        }
    }

    pub fn wait_for_frames(&mut self, win: &mut Window, state: &mut ClientState, frames: usize) -> Result<(), ChaosError> {
        for _ in 0..frames {
            win.update()?;
            self.render(win, state)?;
        }
        Ok(())
    }

    fn draw_spell_cast_info(&self, win: &mut Window, wizard_name: &str, spell_name: Option<&str>, range: Option<u8>) {
        let mut buf = Buffer::new(32, 2);
        buf.draw_text(wizard_name, 0, 0, BrightYellow);
        if let Some(spell_name) = spell_name {
            buf.draw_text(spell_name, wizard_name.len() + 1, 0, BrightGreen);
            if let Some(mut range) = range {
                range /= 2;
                let text = if range > 10 { "20".to_string() } else { range.to_string() };
                buf.draw_text(&text, wizard_name.len() + spell_name.len() + 3, 0, BrightWhite);
            }
        }
        win.buf.draw_buffer(&buf, 32, 22);
    }

    pub fn spell_cast_info(
        &mut self,
        win: &mut Window,
        state: &mut ClientState,
        id: u32,
        spell_name: String,
        range: u8,
    ) -> Result<(), ChaosError> {
        let wizard_name = state.arena.find_wizard(id).name.clone();
        self.draw_spell_cast_info(win, &wizard_name, None, None);
        self.wait_for(win, state, 800)?;
        self.draw_spell_cast_info(win, &wizard_name, Some(&spell_name), None);
        self.wait_for(win, state, 800)?;
        self.draw_spell_cast_info(win, &wizard_name, Some(&spell_name), Some(range));
        self.wait_for(win, state, 800)?;
        Ok(())
    }

    pub fn update_alignment(&self, win: &mut Window, state: &mut ClientState) {
        let mut buf = Buffer::new(32, 2);
        let text = match state.arena.alignment.cmp(&0) {
            Ordering::Less => {
                let symbols = vec!["*"; (state.arena.alignment.abs() / 2) as usize];
                Some(format!("(CHAOS {symbols})", symbols = symbols.join("")))
            }
            Ordering::Greater => {
                let symbols = vec!["^"; (state.arena.alignment / 2) as usize];
                Some(format!("(LAW {symbols})", symbols = symbols.join("")))
            }
            Ordering::Equal => None,
        };
        if let Some(ref text) = text {
            buf.center_text(text, 0, BrightYellow);
        }
        win.buf.draw_buffer(&buf, 0, 22);
    }

    pub fn update_spells(&self, win: &mut Window, state: &mut ClientState) {
        let mut buf = Buffer::new(32, 20);
        for (i, spell) in state.wizard.spells.iter_mut().enumerate() {
            let name_buf = spell.as_name_buffer(state.arena.alignment, state.wizard.stats.spell_ability);
            if i % 2 == 0 {
                buf.draw_buffer(&name_buf, 1, (i / 2) * 2);
            } else {
                buf.draw_buffer(&name_buf, 17, (i / 2) * 2);
            }
        }
        win.buf.draw_buffer(&buf, 0, 2);
    }

    pub fn new_spell(&mut self, win: &mut Window, state: &mut ClientState, id: u32) -> Result<(), ChaosError> {
        let name = state.arena.find_wizard(id).name.clone();
        let text = format!("NEW SPELL FOR {}", name);
        self.set_status(win, &text, BrightYellow);
        self.wait_for(win, state, 800)?;
        self.clear_status(win);
        Ok(())
    }

    fn ask_if_illusion(&mut self, win: &mut Window, state: &mut ClientState) -> Result<Option<bool>, ChaosError> {
        self.set_status(win, "IILLUSION? (PRESS Y OR N)", BrightWhite);
        loop {
            win.update()?;
            if let Some(key) = win.get_yes_or_no_or_cancel() {
                self.clear_status(win);
                match key {
                    Key::Escape => {
                        self.set_status(win, "CHOOSE A SPELL", BrightYellow);
                        return Ok(None);
                    }
                    Key::Y => {
                        return Ok(Some(true));
                    }
                    Key::N => {
                        return Ok(Some(false));
                    }
                    _ => {}
                }
            }
            self.render(win, state)?;
        }
    }

    pub fn choose_spell(&mut self, win: &mut Window, state: &mut ClientState) -> Result<Option<(u32, bool)>, ChaosError> {
        loop {
            win.update()?;
            if win.escape_pressed() {
                return Ok(None);
            }
            if win.mouse_clicked() {
                if let MousePosition::Spell(index) = self.panel.pos {
                    if let Some(spell) = state.wizard.spells.get(index) {
                        if spell.is_creation() {
                            if let Some(illusion) = self.ask_if_illusion(win, state)? {
                                return Ok(Some((index as u32, illusion)));
                            }
                        } else {
                            return Ok(Some((index as u32, false)));
                        }
                    }
                }
            }
            self.render(win, state)?;
        }
    }

    pub fn render(&mut self, win: &mut Window, state: &mut ClientState) -> Result<(), ChaosError> {
        win.buf.draw_buffer(&Buffer::from(&mut state.arena), 33, 1);
        self.panel.render(win, state)?;
        Ok(())
    }

    pub fn choose_tile(
        &mut self,
        win: &mut Window,
        state: &mut ClientState,
        tiles: Vec<(u8, u8)>,
        color: Color,
    ) -> Result<Option<u8>, ChaosError> {
        loop {
            win.update()?;
            if win.escape_pressed() {
                return Ok(None);
            }
            if win.mouse_clicked() {
                if let MousePosition::Tile(mouse_x, mouse_y) = self.panel.pos {
                    if let Some((index, _)) = tiles.iter().enumerate().find(|(_, (x, y))| mouse_x == *x && mouse_y == *y) {
                        return Ok(Some(index as u8));
                    }
                }
            }
            self.render(win, state)?;
            self.render_tiles(win, &tiles, color)?;
        }
    }

    pub fn render_tiles(&self, win: &mut Window, tiles: &[(u8, u8)], color: Color) -> Result<(), ChaosError> {
        for (x, y) in tiles {
            let x = 33 + (x * 2) as usize;
            let y = 1 + (y * 2) as usize;
            win.buf.draw_mouse_cursor(x, y, &MouseCursor::Box, color);
        }
        Ok(())
    }

    fn fx(
        &mut self,
        win: &mut Window,
        state: &mut ClientState,
        buf: &Buffer,
        x: u8,
        y: u8,
        frames: usize,
    ) -> Result<(), ChaosError> {
        let x = 33 + (x * 2) as usize;
        let y = 1 + (y * 2) as usize;
        for _ in 0..frames {
            self.render(win, state)?;
            win.buf.draw_buffer(buf, x, y);
            win.update()?;
        }
        Ok(())
    }

    fn multiple_fx(
        &mut self,
        win: &mut Window,
        state: &mut ClientState,
        buf: &Buffer,
        coords: &[(u8, u8)],
        frames: usize,
    ) -> Result<(), ChaosError> {
        for _ in 0..frames {
            self.render(win, state)?;
            for (x, y) in coords.iter() {
                let x = 33 + (x * 2) as usize;
                let y = 1 + (y * 2) as usize;
                win.buf.draw_buffer(buf, x, y);
            }
            win.update()?;
        }
        Ok(())
    }

    pub fn twirl(&mut self, win: &mut Window, state: &mut ClientState, x: u8, y: u8) -> Result<(), ChaosError> {
        for _ in 0..3 {
            for i in 0..4 {
                let buf = TWIRL_FX.get(i).unwrap();
                self.fx(win, state, buf, x, y, 1)?;
            }
        }
        for i in 4..10 {
            let buf = TWIRL_FX.get(i).unwrap();
            self.fx(win, state, buf, x, y, 1)?;
        }
        Ok(())
    }

    pub fn attack(&mut self, win: &mut Window, state: &mut ClientState, x: u8, y: u8) -> Result<(), ChaosError> {
        for _ in 0..5 {
            for buf in ATTACK_FX.iter() {
                self.fx(win, state, buf, x, y, 1)?;
            }
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn ranged_attack(
        &mut self,
        win: &mut Window,
        state: &mut ClientState,
        sx: u8,
        sy: u8,
        dx: u8,
        dy: u8,
        color: Color,
    ) -> Result<(), ChaosError> {
        self.projectile(win, state, sx, sy, dx, dy, color)?;
        for buf in EXPLODING_CIRCLE_FX.iter() {
            self.fx(win, state, buf, dx, dy, 4)?;
        }
        Ok(())
    }

    pub fn magic_bolt(
        &mut self,
        win: &mut Window,
        state: &mut ClientState,
        sx: u8,
        sy: u8,
        dx: u8,
        dy: u8,
    ) -> Result<(), ChaosError> {
        self.fireball(win, state, sx, sy, dx, dy)?;
        for buf in EXPLODING_CIRCLE_FX.iter() {
            self.fx(win, state, buf, dx, dy, 4)?;
        }
        Ok(())
    }

    pub fn dragon_burn(&mut self, win: &mut Window, state: &mut ClientState, x: u8, y: u8) -> Result<(), ChaosError> {
        for buf in DRAGON_BURN_FX.iter() {
            self.fx(win, state, buf, x, y, 4)?;
        }
        Ok(())
    }

    pub fn dragon_ranged_attack(
        &mut self,
        win: &mut Window,
        state: &mut ClientState,
        sx: u8,
        sy: u8,
        dx: u8,
        dy: u8,
    ) -> Result<(), ChaosError> {
        self.fireballs(win, state, sx, sy, dx, dy)?;
        self.dragon_burn(win, state, dx, dy)?;
        Ok(())
    }

    pub fn explosion(&mut self, win: &mut Window, state: &mut ClientState, x: u8, y: u8) -> Result<(), ChaosError> {
        for buf in EXPLOSION_FX.iter() {
            self.fx(win, state, buf, x, y, 4)?;
        }
        Ok(())
    }

    pub fn explosions(&mut self, win: &mut Window, state: &mut ClientState, coords: Vec<(u8, u8)>) -> Result<(), ChaosError> {
        for buf in EXPLOSION_FX.iter() {
            self.multiple_fx(win, state, buf, &coords, 4)?;
        }
        Ok(())
    }

    pub fn ask_for_dismount(&mut self, win: &mut Window, state: &mut ClientState) -> Result<Option<bool>, ChaosError> {
        loop {
            win.update()?;
            if let Some(key) = win.get_yes_or_no_or_cancel() {
                match key {
                    Key::Escape => {
                        return Ok(None);
                    }
                    Key::Y => {
                        return Ok(Some(true));
                    }
                    Key::N => {
                        return Ok(Some(false));
                    }
                    _ => {}
                }
            }
            self.render(win, state)?;
        }
    }

    pub fn results(&mut self, win: &mut Window, players: Vec<Player>) -> Result<(), ChaosError> {
        win.buf.clear();
        if players.len() > 1 {
            loop {
                for color_index in 1..=7 {
                    let color = Color::try_from(color_index + 8).expect("invalid color");
                    win.buf.screen_border("PRESS ANY KEY", color, Black);
                    let title_color = Color::try_from((color_index + 1) % 7 + 9).expect("invalid color");
                    win.buf.center_text("THE CONTEST IS DRAWN BETWEEN", 2, title_color);
                    for (player_index, player) in players.iter().enumerate() {
                        let player_color =
                            Color::try_from((color_index + 1 + player_index as u8) % 7 + 9).expect("invalid color");
                        let x = (96 - player.name.len()) / 2;
                        win.buf.draw_text(&player.name, x, 6 + player_index * 2, player_color);
                    }
                    for _ in 0..8 {
                        win.update()?;
                        if win.any_key_pressed() {
                            return Ok(());
                        }
                    }
                }
            }
        } else {
            loop {
                for color_index in 1..=7 {
                    let color = Color::try_from(color_index + 8).expect("invalid color");
                    win.buf.screen_border("PRESS ANY KEY", color, Black);
                    let title_color = Color::try_from((color_index + 1) % 7 + 9).expect("invalid color");
                    win.buf.center_text("THE WINNER IS:", 4, title_color);
                    let lawful_border_color = Color::try_from((color_index + 2) % 7 + 9).expect("invalid color");
                    win.buf.center_text("^^^^^^^^^^^^^^^^", 8, lawful_border_color);
                    win.buf.center_text("^              ^", 10, lawful_border_color);
                    win.buf.center_text("^              ^", 12, lawful_border_color);
                    win.buf.center_text("^              ^", 14, lawful_border_color);
                    win.buf.center_text("^^^^^^^^^^^^^^^^", 16, lawful_border_color);
                    let player = players.first().expect("invalid index");
                    let player_color = Color::try_from((color_index + 3) % 7 + 9).expect("invalid color");
                    win.buf.center_text(&player.name, 12, player_color);
                    for _ in 0..8 {
                        win.update()?;
                        if win.any_key_pressed() {
                            return Ok(());
                        }
                    }
                }
            }
        }
    }

    pub fn spell_ray(
        &mut self,
        win: &mut Window,
        state: &mut ClientState,
        sx: u8,
        sy: u8,
        dx: u8,
        dy: u8,
    ) -> Result<(), ChaosError> {
        let points = Arena::line_coords(sx, sy, dx, dy);
        for start in (0..points.len() + 30).step_by(4) {
            win.update()?;
            self.render(win, state)?;
            let mut buf = Buffer::from(&state.arena);
            buf.draw_spell_line(&points, start);
            win.buf.draw_buffer(&buf, 33, 1);
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn projectile(
        &mut self,
        win: &mut Window,
        state: &mut ClientState,
        sx: u8,
        sy: u8,
        dx: u8,
        dy: u8,
        color: Color,
    ) -> Result<(), ChaosError> {
        let points = Arena::line_coords(sx, sy, dx, dy);
        for start in (0..points.len() + 10).step_by(4) {
            win.update()?;
            self.render(win, state)?;
            let mut buf = Buffer::from(&state.arena);
            buf.draw_projectile(&points, start, color);
            win.buf.draw_buffer(&buf, 33, 1);
        }
        Ok(())
    }

    pub fn fireballs(
        &mut self,
        win: &mut Window,
        state: &mut ClientState,
        sx: u8,
        sy: u8,
        dx: u8,
        dy: u8,
    ) -> Result<(), ChaosError> {
        let points = Arena::line_coords(sx, sy, dx, dy);
        for start in (0..points.len() + 30).step_by(4) {
            win.update()?;
            self.render(win, state)?;
            let mut buf = Buffer::from(&state.arena);
            buf.draw_fireballs(&points, start);
            win.buf.draw_buffer(&buf, 33, 1);
        }
        Ok(())
    }

    pub fn fireball(
        &mut self,
        win: &mut Window,
        state: &mut ClientState,
        sx: u8,
        sy: u8,
        dx: u8,
        dy: u8,
    ) -> Result<(), ChaosError> {
        for (x, y) in Arena::line_coords(sx, sy, dx, dy).into_iter().step_by(4) {
            win.update()?;
            self.render(win, state)?;
            let mut buf = Buffer::from(&state.arena);
            buf.draw_fireball(x, y, BrightYellow);
            win.buf.draw_buffer(&buf, 33, 1);
        }
        Ok(())
    }

    pub fn lightning(
        &mut self,
        win: &mut Window,
        state: &mut ClientState,
        sx: u8,
        sy: u8,
        dx: u8,
        dy: u8,
    ) -> Result<(), ChaosError> {
        let points = Arena::line_coords(sx, sy, dx, dy);
        for start in (0..points.len() + 30).step_by(4) {
            win.update()?;
            self.render(win, state)?;
            let mut buf = Buffer::from(&state.arena);
            buf.draw_lightning(&points, start);
            win.buf.draw_buffer(&buf, 33, 1);
        }
        for buf in EXPLODING_CIRCLE_FX.iter() {
            self.fx(win, state, buf, dx, dy, 4)?;
        }
        Ok(())
    }

    pub fn flash_attack(&mut self, win: &mut Window, state: &mut ClientState, x: u8, y: u8) -> Result<(), ChaosError> {
        let tile = state.arena.get(x, y);
        let buf = if let Some(ref creation) = tile.creation {
            creation.current_bytes()
        } else if let Some(ref wizard) = tile.wizard {
            wizard.current_bytes()
        } else {
            unreachable!("invalid tile");
        };
        let bufs = (9..=15)
            .map(|color_index| {
                let color = Color::try_from(color_index).expect("invalid color");
                let frame = Frame {
                    fg: color,
                    bg: None,
                    bytes: buf,
                };
                Buffer::from(&frame)
            })
            .collect::<Vec<_>>();
        for _ in 0..6 {
            for buf in bufs.iter() {
                self.fx(win, state, buf, x, y, 4)?;
            }
        }
        Ok(())
    }

    pub fn wizard_death(&mut self, win: &mut Window, state: &mut ClientState, id: u32) -> Result<(), ChaosError> {
        let bytes = state.arena.find_wizard(id).current_bytes();
        let (x, y) = state.arena.find_wizard_pos(id);
        let x = x as isize * 2;
        let y = y as isize * 2;
        let wiz_bufs = (0..8)
            .map(|i| {
                Buffer::from(&Frame {
                    fg: WizardColor::try_from(i).expect("invalid wizard color").into(),
                    bg: None,
                    bytes,
                })
            })
            .collect::<Vec<_>>();
        let mut buf = Buffer::from(&state.arena);
        for outer_i in 0..128 {
            for inner_i in 0..28 {
                let index = ((inner_i as isize - outer_i).abs() % 8) as usize;
                let wiz_buf = wiz_bufs.get(index).expect("invalid index");
                let right = x + inner_i as isize;
                if right <= 29 {
                    buf.draw_buffer(wiz_buf, right as usize, y as usize);
                }
                let left = x - inner_i as isize;
                if left >= 0 {
                    buf.draw_buffer(wiz_buf, left as usize, y as usize);
                }
                let up = y - inner_i as isize;
                if up >= 0 {
                    buf.draw_buffer(wiz_buf, x as usize, up as usize);
                }
                let down = y + inner_i as isize;
                if down <= 19 {
                    buf.draw_buffer(wiz_buf, x as usize, down as usize);
                }
                if right <= 29 && up >= 0 {
                    buf.draw_buffer(wiz_buf, right as usize, up as usize);
                }
                if right <= 29 && down <= 19 {
                    buf.draw_buffer(wiz_buf, right as usize, down as usize);
                }
                if left >= 0 && up >= 0 {
                    buf.draw_buffer(wiz_buf, left as usize, up as usize);
                }
                if left >= 0 && down <= 19 {
                    buf.draw_buffer(wiz_buf, left as usize, down as usize);
                }
            }
            win.update()?;
            self.render(win, state)?;
            win.buf.draw_buffer(&buf, 33, 1);
        }
        let coords = state.arena.get_topmost_creations_and_corpses_coords(id);
        self.explosions(win, state, coords)?;
        state.arena.kill_wizard_and_creations(id);
        Ok(())
    }

    pub fn set_status(&mut self, win: &mut Window, text: &str, color: Color) {
        win.buf.clear_area(32, 22, 32, 2);
        win.buf.draw_text(text, 32, 22, color);
    }

    pub fn multi_color_status(&mut self, win: &mut Window, content: &[(&str, Color)]) {
        win.buf.clear_area(32, 22, 32, 2);
        let mut x = 32;
        for (text, color) in content {
            win.buf.draw_text(text, x, 22, *color);
            x += text.len();
        }
    }

    pub fn clear_status(&mut self, win: &mut Window) {
        win.buf.clear_area(32, 22, 32, 2);
    }
}
