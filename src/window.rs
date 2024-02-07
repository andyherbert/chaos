use crate::error::ChaosError;
use crate::gfx::buffer::Buffer;
use crate::gfx::color::Color;
pub use minifb::Key;
use minifb::{KeyRepeat, MouseButton, MouseMode, Scale, Window as MiniFBWindow, WindowOptions};
use std::ops::RangeInclusive;
use std::time::{Duration, Instant};

pub struct Window {
    pub win: MiniFBWindow,
    pub buf: Buffer,
}

impl Window {
    pub fn new() -> Result<Self, ChaosError> {
        let name = env!("CARGO_PKG_DESCRIPTION");
        let width = 768;
        let height = 192;
        let opts = WindowOptions {
            scale: Scale::X2,
            ..WindowOptions::default()
        };
        let mut win = MiniFBWindow::new(name, width, height, opts)?;
        win.limit_update_rate(Some(Duration::from_millis(1000 / 50)));
        let buf = Buffer::new(width / 8, height / 8);
        Ok(Self { win, buf })
    }

    pub fn update(&mut self) -> Result<(), ChaosError> {
        if !self.win.is_open() || self.check_for_quit() {
            Err(ChaosError::Quit)
        } else {
            self.win.update_with_buffer(&self.buf.data, self.buf.width, self.buf.height)?;
            Ok(())
        }
    }

    pub fn wait(&mut self, ms: u128) -> Result<(), ChaosError> {
        let now = Instant::now();
        while now.elapsed().as_millis() < ms {
            self.win.update();
        }
        Ok(())
    }

    pub fn mouse_coords(&self) -> Option<(usize, usize)> {
        match self.win.get_mouse_pos(MouseMode::Discard) {
            Some((x, y)) => Some((x as usize / 8, y as usize / 8)),
            _ => None,
        }
    }

    pub fn mouse_clicked(&self) -> bool {
        self.win.get_mouse_down(MouseButton::Left)
    }

    pub fn quit(&self) -> Result<(), ChaosError> {
        Err(ChaosError::Quit)
    }

    #[cfg(not(target_os = "macos"))]
    pub fn check_for_quit(&self) -> bool {
        use Key::*;
        self.win.is_key_down(LeftAlt) || self.win.is_key_down(RightAlt) && self.win.is_key_down(F4)
    }

    #[cfg(target_os = "macos")]
    pub fn check_for_quit(&self) -> bool {
        use Key::*;
        (self.win.is_key_down(LeftSuper) || self.win.is_key_down(RightSuper))
            && (self.win.is_key_down(Q) || self.win.is_key_down(W))
    }

    pub fn wait_for_any_key(&mut self) -> Result<(), ChaosError> {
        loop {
            self.update()?;
            if !self.win.get_keys_pressed(KeyRepeat::No).is_empty() {
                return Ok(());
            }
        }
    }

    pub fn wait_for_number(&mut self, range: RangeInclusive<isize>) -> Result<Option<isize>, ChaosError> {
        loop {
            self.update()?;
            for key in self.win.get_keys_pressed(KeyRepeat::No) {
                use Key::*;
                let digit = (key as isize) - (Key0 as isize);
                if Escape == key {
                    return Ok(None);
                } else if range.contains(&digit) {
                    return Ok(Some(digit));
                }
            }
        }
    }

    pub fn wizard_name(
        &mut self,
        mut name: String,
        x: usize,
        y: usize,
        max_len: usize,
        fg: Color,
    ) -> Result<Option<String>, ChaosError> {
        use Key::*;
        loop {
            self.update()?;
            self.buf.clear_area(x, y, max_len + 1, 2);
            for key in self.win.get_keys_pressed(KeyRepeat::Yes) {
                let discriminant = key as isize;
                match discriminant {
                    0..=9 if name.len() < max_len => {
                        let ch = (discriminant + 48) as u8;
                        name.push(ch as char);
                    }
                    10..=35 if name.len() < max_len => {
                        let ch = if self.win.is_key_down(LeftShift) || self.win.is_key_down(RightShift) {
                            discriminant + 55
                        } else {
                            discriminant + 87
                        } as u8;
                        name.push(ch as char);
                    }
                    _ => match key {
                        Escape => {
                            return Ok(None);
                        }
                        Backspace if !name.is_empty() => {
                            name.pop();
                        }
                        Enter if !name.is_empty() => {
                            self.buf.draw_text(&name, x, y, fg);
                            return Ok(Some(name));
                        }
                        Space if !name.is_empty() => {
                            name.push(' ');
                        }
                        _ => {}
                    },
                }
            }
            self.buf.draw_text(&name, x, y, fg);
            self.buf.draw_cursor(x + name.len(), y, fg);
        }
    }

    pub fn host_entry(
        &mut self,
        mut host: String,
        x: usize,
        y: usize,
        max_len: usize,
        fg: Color,
    ) -> Result<Option<String>, ChaosError> {
        loop {
            self.update()?;
            self.buf.clear_area(x, y, max_len + 1, 2);
            for key in self.win.get_keys_pressed(KeyRepeat::Yes) {
                let discriminant = key as isize;
                match discriminant {
                    0..=9 if host.len() < max_len => {
                        let ch = (discriminant + 48) as u8;
                        host.push(ch as char);
                    }
                    10..=35 if host.len() < max_len => {
                        let ch = if self.win.is_key_down(Key::LeftShift) || self.win.is_key_down(Key::RightShift) {
                            discriminant + 55
                        } else {
                            discriminant + 87
                        } as u8;
                        host.push(ch as char);
                    }
                    _ => match key {
                        Key::Escape => {
                            return Ok(None);
                        }
                        Key::Period if host.len() < max_len => {
                            host.push('.');
                        }
                        Key::Backspace if !host.is_empty() => {
                            host.pop();
                        }
                        Key::Enter if !host.is_empty() => {
                            self.buf.draw_text(&host, x, y, fg);
                            return Ok(Some(host.clone()));
                        }
                        Key::Space if !host.is_empty() => host.push(' '),
                        _ => {}
                    },
                }
            }
            self.buf.draw_text(&host, x, y, fg);
            self.buf.draw_cursor(x + host.len(), y, fg);
        }
    }

    pub fn port_entry(
        &mut self,
        port: usize,
        x: usize,
        y: usize,
        max_len: usize,
        fg: Color,
    ) -> Result<Option<usize>, ChaosError> {
        let mut string = port.to_string();
        loop {
            self.update()?;
            self.buf.clear_area(x, y, max_len + 1, 2);
            for key in self.win.get_keys_pressed(KeyRepeat::Yes) {
                let discriminant = key as isize;
                match discriminant {
                    0..=9 if string.len() < max_len => {
                        let ch = (discriminant + 48) as u8;
                        string.push(ch as char);
                    }
                    _ => match key {
                        Key::Escape => {
                            return Ok(None);
                        }
                        Key::Enter if !string.is_empty() => {
                            self.buf.draw_text(&string, x, y, fg);
                            let port = string.parse().expect("parsing port");
                            return Ok(Some(port));
                        }
                        Key::Backspace if !string.is_empty() => {
                            string.pop();
                        }
                        _ => {}
                    },
                }
            }
            self.buf.draw_text(&string, x, y, fg);
            self.buf.draw_cursor(x + string.len(), y, fg);
        }
    }

    pub fn get_yes_or_no_or_cancel(&mut self) -> Option<Key> {
        for key in self.win.get_keys_pressed(KeyRepeat::No) {
            use Key::*;
            match key {
                Y => return Some(Y),
                N => return Some(N),
                Escape => return Some(Escape),
                _ => continue,
            }
        }
        None
    }

    pub fn escape_pressed(&mut self) -> bool {
        self.win.is_key_pressed(Key::Escape, KeyRepeat::No)
    }

    pub fn is_down_pressed(&mut self) -> bool {
        self.win.is_key_pressed(Key::Down, KeyRepeat::Yes)
    }

    pub fn is_up_pressed(&mut self) -> bool {
        self.win.is_key_pressed(Key::Up, KeyRepeat::Yes)
    }

    pub fn any_key_pressed(&mut self) -> bool {
        !self.win.get_keys_pressed(KeyRepeat::No).is_empty()
    }
}
