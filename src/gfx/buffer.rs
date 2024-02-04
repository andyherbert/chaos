use super::color::{Color, ColorIndextoColorTuple, ColorTupleToColorIndex};
use crate::config::Player;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use std::vec;

static CURSORS: &[u8; 128] = include_bytes!("bin/cursors.bin");

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MouseCursor {
    Spell,
    Box,
    Wings,
    Ranged,
}

impl MouseCursor {
    pub fn as_bytes(&self) -> &[u8] {
        match self {
            Self::Spell => &CURSORS[0..32],
            Self::Box => &CURSORS[32..64],
            Self::Wings => &CURSORS[64..96],
            Self::Ranged => &CURSORS[96..],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Buffer {
    pub data: Vec<u32>,
    pub width: usize,
    pub height: usize,
}

impl Buffer {
    pub fn new(width: usize, height: usize) -> Self {
        let black = Color::Black.into();
        Self {
            data: vec![black; (width * 8) * (height * 8)],
            width: width * 8,
            height: height * 8,
        }
    }

    fn draw_bytes(&mut self, bytes: &[u8], x: usize, y: usize, fg: Color, bg: Option<Color>) {
        let fg = fg.into();
        let bg = bg.map(|bg| bg.into());
        let mut index = (self.width * (y * 8)) + x * 8;
        for mut byte in bytes.iter().copied() {
            for _ in 0..8 {
                if let Some(data) = self.data.get_mut(index) {
                    if (byte & 0b1000_0000) == 0b1000_0000 {
                        *data = fg;
                    } else if let Some(bg) = bg {
                        *data = bg;
                    };
                }
                byte <<= 1;
                index += 1;
            }
            index += self.width - 8;
        }
    }

    pub fn from_bytes(bytes: &[u8], fg: Color, bg: Option<Color>) -> Self {
        let mut buf = Buffer::new(1, bytes.len() / 8);
        buf.draw_bytes(bytes, 0, 0, fg, bg);
        buf
    }

    pub fn draw_shorts(&mut self, bytes: &[u8], x: usize, y: usize, fg: Color, bg: Option<Color>) {
        let fg = fg.into();
        let bg = bg.map(|bg| bg.into());
        let mut index = (self.width * (y * 8)) + x * 8;
        for slice in bytes.chunks(2) {
            let mut short = u16::from_be_bytes(slice.try_into().expect("shorts in bytes"));
            for _ in 0..16 {
                if let Some(data) = self.data.get_mut(index) {
                    if (short & 0b1000_0000_0000_0000) == 0b1000_0000_0000_0000 {
                        *data = fg;
                    } else if let Some(bg) = bg {
                        *data = bg;
                    };
                }
                short <<= 1;
                index += 1;
            }
            index += self.width - 16;
        }
    }

    pub fn from_shorts(bytes: &[u8], fg: Color, bg: Option<Color>) -> Self {
        let mut buf = Buffer::new(2, bytes.len() / 16);
        buf.draw_shorts(bytes, 0, 0, fg, bg);
        buf
    }

    pub fn fill_area(&mut self, x: usize, y: usize, width: usize, height: usize, col: Color) {
        let col = col.into();
        for y in y * 8..(y + height) * 8 {
            let start = (y * self.width) + x * 8;
            if let Some(slice) = self.data.get_mut(start..start + width * 8) {
                slice.fill(col);
            }
        }
    }

    pub fn clear_area(&mut self, x: usize, y: usize, width: usize, height: usize) {
        self.fill_area(x, y, width, height, Color::Black);
    }

    pub fn clear(&mut self) {
        self.data.fill(Color::Black.into());
    }

    pub fn crop(&self, x: usize, y: usize, width: usize, height: usize) -> Self {
        let width = width * 8;
        let height = height * 8;
        let mut data = vec![0; width * height];
        let mut index = ((y * 8) * self.width) + (x * 8);
        for dst in data.chunks_mut(width) {
            if let Some(data) = self.data.get(index..index + width) {
                dst.copy_from_slice(data);
            }
            index += self.width;
        }
        Self { data, width, height }
    }

    pub fn draw_buffer(&mut self, buf: &Buffer, x: usize, y: usize) {
        let mut index = (self.width * (y * 8)) + (x * 8);
        for src in buf.data.chunks(buf.width) {
            if let Some(data) = self.data.get_mut(index..index + buf.width) {
                data.copy_from_slice(src);
            }
            index += self.width;
        }
    }
}

static BORDER: &[u8; 64] = include_bytes!("bin/border.bin");

struct Borders {
    top: Buffer,
    bottom: Buffer,
    left: Buffer,
    right: Buffer,
    top_left: Buffer,
    top_right: Buffer,
    bottom_left: Buffer,
    bottom_right: Buffer,
}

impl Borders {
    pub fn new(fg: Color, bg: Color) -> Self {
        Self {
            top: Buffer::from_bytes(&BORDER[0..8], fg, Some(bg)),
            bottom: Buffer::from_bytes(&BORDER[8..16], fg, Some(bg)),
            left: Buffer::from_bytes(&BORDER[16..24], fg, Some(bg)),
            right: Buffer::from_bytes(&BORDER[24..32], fg, Some(bg)),
            top_left: Buffer::from_bytes(&BORDER[32..40], fg, Some(bg)),
            top_right: Buffer::from_bytes(&BORDER[40..48], fg, Some(bg)),
            bottom_left: Buffer::from_bytes(&BORDER[48..56], fg, Some(bg)),
            bottom_right: Buffer::from_bytes(&BORDER[56..64], fg, Some(bg)),
        }
    }
}

lazy_static! {
    static ref BORDERS: Vec<Borders> = {
        let mut vec = Vec::with_capacity(256);
        for i in 0..=255 {
            let (fg, bg) = i.to_color_tuple().expect("color");
            vec.push(Borders::new(fg, bg));
        }
        vec
    };
}

impl Buffer {
    pub fn border(&mut self, x: usize, y: usize, width: usize, height: usize, fg: Color, bg: Color) {
        let borders = &BORDERS[(fg, bg).to_color_index()];
        for x in x + 1..x + width - 1 {
            self.draw_buffer(&borders.top, x, y);
            self.draw_buffer(&borders.bottom, x, y + height - 1);
        }
        for y in y + 1..y + height - 1 {
            self.draw_buffer(&borders.left, x, y);
            self.draw_buffer(&borders.right, x + width - 1, y);
        }
        self.draw_buffer(&borders.top_left, x, y);
        self.draw_buffer(&borders.top_right, x + width - 1, y);
        self.draw_buffer(&borders.bottom_left, x, y + height - 1);
        self.draw_buffer(&borders.bottom_right, x + width - 1, y + height - 1);
    }

    pub fn screen_border(&mut self, text: &str, fg: Color, bg: Color) {
        let width = self.width / 8;
        let height = (self.height / 8) - 2;
        self.border(0, 0, width, height, fg, bg);
        self.fill_area(0, height, width, 2, fg);
        self.center_text_with_bg(text, height, bg, fg);
    }
}

static TEXT_CHARS: &[u8; 1552] = include_bytes!("bin/text_characters.bin");

lazy_static! {
    static ref CHARMAP: Vec<Vec<Buffer>> = {
        let mut vec = Vec::with_capacity(256);
        for color_index in 0..=255 {
            let (fg, bg) = color_index.to_color_tuple().expect("color");
            let mut char_vec = Vec::with_capacity(96);
            for char_index in 0..=95 {
                let index = char_index * 16;
                let buf = Buffer::from_bytes(&TEXT_CHARS[index..index + 16], fg, Some(bg));
                char_vec.push(buf);
            }
            vec.push(char_vec);
        }
        vec
    };
}

impl Buffer {
    pub fn draw_text_with_bg(&mut self, text: &str, x: usize, y: usize, fg: Color, bg: Color) {
        for (ln, text) in text.split('\n').enumerate() {
            for (col, ch) in text.chars().enumerate() {
                let char_code = if ch == '©' {
                    95
                } else {
                    match u8::try_from(ch) {
                        Ok(0..=31 | 128..=255) | Err(_) => 0,
                        Ok(ascii_code) => (ascii_code as usize) - 32,
                    }
                };
                if let Some(vec) = CHARMAP.get((fg, bg).to_color_index()) {
                    if let Some(buf) = vec.get(char_code) {
                        self.draw_buffer(buf, x + col, y + (ln * 2));
                    }
                }
            }
        }
    }

    pub fn draw_text(&mut self, text: &str, x: usize, y: usize, fg: Color) {
        self.draw_text_with_bg(text, x, y, fg, Color::Black);
    }

    pub fn center_text_with_bg(&mut self, text: &str, y: usize, fg: Color, bg: Color) {
        self.draw_text_with_bg(text, (self.width / 8 - text.len()) / 2, y, fg, bg);
    }

    pub fn center_text(&mut self, text: &str, y: usize, fg: Color) {
        self.center_text_with_bg(text, y, fg, Color::Black);
    }

    pub fn center_player(&mut self, player: &Player, y: usize, fg: Color, render_wizard: bool) {
        self.center_text(&player.name, y, fg);
        if render_wizard {
            let buf = Buffer::from(player);
            let x = (self.width / 8 / 2) + (player.name.len() / 2);
            self.draw_buffer(&buf, x, y);
        }
    }

    pub fn draw_cursor(&mut self, x: usize, y: usize, fg: Color) {
        self.draw_bytes(&TEXT_CHARS[1536..], x, y, fg, None);
    }

    pub fn draw_mouse_cursor(&mut self, x: usize, y: usize, cursor: &MouseCursor, fg: Color) {
        self.draw_shorts(cursor.as_bytes(), x, y, fg, None);
    }
}

static LOADING_SCREEN: &[u8; 6912] = include_bytes!("bin/loading_screen.bin");

lazy_static! {
    static ref SCREEN_BUFFER: Buffer = {
        let width = 256;
        let height = 192;
        let mut data = vec![0; width * height];
        let mut index = 0;
        if let Some(screen_chunk) = LOADING_SCREEN.get(0..6144) {
            for (attrib_y, chunk) in screen_chunk.chunks_exact(32).enumerate() {
                for (attrib_x, mut byte) in chunk.iter().copied().enumerate() {
                    let attrib_index = 6144 + (attrib_y / 8 * 32) + attrib_x;
                    for _ in 0..8 {
                        if let Some(attrib) = LOADING_SCREEN.get(attrib_index) {
                            let bright = (*attrib & 0b0100_0000) == 0b0100_0000;
                            let color = if (byte & 0b1000_0000) == 0b1000_0000 {
                                *attrib & 0b0000_0111
                            } else {
                                (*attrib >> 3) & 0b0000_0111
                            };
                            if let Some(data) = data.get_mut(index) {
                                *data = if bright {
                                    Color::try_from(color + 8)
                                } else {
                                    Color::try_from(color)
                                }
                                .expect("color")
                                .into();
                            }
                            byte <<= 1;
                            index += 1;
                        }
                    }
                }
            }
        }
        Buffer { data, width, height }
    };
    pub static ref LOGO: Buffer = SCREEN_BUFFER.crop(1, 4, 17, 4);
    pub static ref SNAKE: Buffer = SCREEN_BUFFER.crop(0, 9, 32, 15);
}

impl Buffer {
    #[inline]
    fn put_pixel(&mut self, x: usize, y: usize, color: Color) {
        if let Some(data) = self.data.get_mut((y * self.width) + x) {
            *data = color.into();
        }
    }

    pub fn get_pixel(&self, x: usize, y: usize) -> Option<u32> {
        self.data.get((y * self.width) + x).copied()
    }

    pub fn draw_spell_cross(&mut self, x: usize, y: usize, color: Color) {
        self.put_pixel(x, y - 1, color);
        self.put_pixel(x - 1, y, color);
        self.put_pixel(x, y, color);
        self.put_pixel(x + 1, y, color);
        self.put_pixel(x, y + 1, color);
    }

    pub fn draw_fireball(&mut self, x: usize, y: usize, color: Color) {
        self.draw_spell_cross(x, y, color);
        self.put_pixel(x, y - 3, color);
        self.put_pixel(x + 2, y - 2, color);
        self.put_pixel(x + 3, y, color);
        self.put_pixel(x + 2, y + 2, color);
        self.put_pixel(x, y + 3, color);
        self.put_pixel(x - 2, y + 2, color);
        self.put_pixel(x - 3, y, color);
        self.put_pixel(x - 2, y - 2, color);
    }

    pub fn draw_fireballs(&mut self, points: &[(usize, usize)], start: usize) {
        for i in (start as isize - 30..start as isize).step_by(4) {
            if i > 0 {
                if let Some((x, y)) = points.get(i as usize) {
                    self.draw_fireball(*x, *y, Color::BrightYellow);
                }
            }
        }
        if start >= 30 {
            if let Some((x, y)) = points.get(start - 30) {
                self.draw_spell_cross(*x, *y, Color::Black);
            }
        }
    }

    pub fn draw_lightning(&mut self, points: &[(usize, usize)], start: usize) {
        for i in (start as isize - 30..start as isize).step_by(4) {
            if i > 0 {
                if let Some((x, y)) = points.get(i as usize) {
                    self.draw_fireball(*x, *y, Color::BrightWhite);
                }
            }
        }
        if start >= 30 {
            if let Some((x, y)) = points.get(start - 30) {
                self.draw_spell_cross(*x, *y, Color::Black);
            }
        }
    }

    pub fn draw_spell_line(&mut self, points: &[(usize, usize)], start: usize) {
        for i in start as isize - 30..start as isize {
            if i > 0 {
                if let Some((x, y)) = points.get(i as usize) {
                    self.draw_spell_cross(*x, *y, Color::BrightCyan);
                }
            }
        }
        if start >= 30 {
            if let Some((x, y)) = points.get(start - 30) {
                self.draw_spell_cross(*x, *y, Color::Black);
            }
        }
    }

    pub fn draw_projectile(&mut self, points: &[(usize, usize)], start: usize, color: Color) {
        for i in start as isize - 10..start as isize {
            if i > 0 {
                if let Some((x, y)) = points.get(i as usize) {
                    self.put_pixel(*x, *y, color)
                }
            }
        }
    }
}
