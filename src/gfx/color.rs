use crate::error::ChaosError;
use serde::{Deserialize, Serialize};
use std::{error, fmt};

#[derive(Clone, Debug, Copy, Default, Serialize, Deserialize)]
pub enum Color {
    #[default]
    Black,
    Blue,
    Red,
    Magenta,
    Green,
    Cyan,
    Yellow,
    White,
    BrightBlack,
    BrightBlue,
    BrightRed,
    BrightMagenta,
    BrightGreen,
    BrightCyan,
    BrightYellow,
    BrightWhite,
}

const fn rgb_to_u32(r: u8, g: u8, b: u8) -> u32 {
    ((r as u32) << 16) + ((g as u32) << 8) + (b as u32)
}

impl From<Color> for u32 {
    fn from(value: Color) -> Self {
        match value {
            Color::Black => rgb_to_u32(9, 9, 9),
            Color::Blue => rgb_to_u32(29, 0, 166),
            Color::Red => rgb_to_u32(140, 0, 0),
            Color::Magenta => rgb_to_u32(157, 0, 161),
            Color::Green => rgb_to_u32(0, 143, 0),
            Color::Cyan => rgb_to_u32(0, 166, 168),
            Color::Yellow => rgb_to_u32(182, 180, 0),
            Color::White => rgb_to_u32(204, 204, 204),
            Color::BrightBlack => rgb_to_u32(9, 9, 9),
            Color::BrightBlue => rgb_to_u32(34, 0, 186),
            Color::BrightRed => rgb_to_u32(164, 0, 0),
            Color::BrightMagenta => rgb_to_u32(186, 0, 189),
            Color::BrightGreen => rgb_to_u32(0, 175, 0),
            Color::BrightCyan => rgb_to_u32(0, 204, 205),
            Color::BrightYellow => rgb_to_u32(225, 224, 0),
            Color::BrightWhite => rgb_to_u32(255, 255, 255),
        }
    }
}

impl TryFrom<u8> for Color {
    type Error = ColorError;

    fn try_from(value: u8) -> core::result::Result<Self, Self::Error> {
        match value {
            0 => Ok(Color::Black),
            1 => Ok(Color::Blue),
            2 => Ok(Color::Red),
            3 => Ok(Color::Magenta),
            4 => Ok(Color::Green),
            5 => Ok(Color::Cyan),
            6 => Ok(Color::Yellow),
            7 => Ok(Color::White),
            8 => Ok(Color::BrightBlack),
            9 => Ok(Color::BrightBlue),
            10 => Ok(Color::BrightRed),
            11 => Ok(Color::BrightMagenta),
            12 => Ok(Color::BrightGreen),
            13 => Ok(Color::BrightCyan),
            14 => Ok(Color::BrightYellow),
            15 => Ok(Color::BrightWhite),
            _ => Err(ColorError::InvalidColor),
        }
    }
}

impl From<Color> for usize {
    fn from(value: Color) -> Self {
        match value {
            Color::Black => 0,
            Color::Blue => 1,
            Color::Red => 2,
            Color::Magenta => 3,
            Color::Green => 4,
            Color::Cyan => 5,
            Color::Yellow => 6,
            Color::White => 7,
            Color::BrightBlack => 8,
            Color::BrightBlue => 9,
            Color::BrightRed => 10,
            Color::BrightMagenta => 11,
            Color::BrightGreen => 12,
            Color::BrightCyan => 13,
            Color::BrightYellow => 14,
            Color::BrightWhite => 15,
        }
    }
}

#[derive(Debug)]
pub enum ColorError {
    InvalidColor,
}

impl From<ColorError> for ChaosError {
    fn from(_value: ColorError) -> Self {
        Self::GameError
    }
}

impl fmt::Display for ColorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ColorError::InvalidColor => write!(f, "Invalid color"),
        }
    }
}

pub trait ColorTupleToColorIndex {
    fn to_color_index(&self) -> usize;
}

impl ColorTupleToColorIndex for (Color, Color) {
    fn to_color_index(&self) -> usize {
        let (fg, bg) = self;
        let fg_index = usize::from(*fg);
        let bg_index = usize::from(*bg);
        (fg_index << 4) + bg_index
    }
}

pub trait ColorIndextoColorTuple {
    type Error;
    fn to_color_tuple(&self) -> Result<(Color, Color), Self::Error>;
}

impl ColorIndextoColorTuple for usize {
    type Error = ColorError;

    fn to_color_tuple(&self) -> Result<(Color, Color), Self::Error> {
        let fg_index = ((self & 0b11110000) >> 4) as u8;
        let bg_index = (self & 0b00001111) as u8;
        let fg = Color::try_from(fg_index)?;
        let bg = Color::try_from(bg_index)?;
        Ok((fg, bg))
    }
}

impl error::Error for ColorError {}
