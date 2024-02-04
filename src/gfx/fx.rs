use crate::gfx::buffer::Buffer;
use crate::gfx::color::Color::*;
use lazy_static::lazy_static;

static ATTACK_BYTES: &[u8; 128] = include_bytes!("bin/fx/attack.bin");
static DRAGON_BURN_BYTES: &[u8; 288] = include_bytes!("bin/fx/dragon_burn.bin");
static EXPLODING_CIRCLE_BYTES: &[u8; 288] = include_bytes!("bin/fx/exploding_circle.bin");
static EXPLOSION_BYTES: &[u8; 224] = include_bytes!("bin/fx/explosion.bin");
static TWIRL_BYTES: &[u8; 320] = include_bytes!("bin/fx/twirl.bin");

lazy_static! {
    pub static ref ATTACK_FX: [Buffer; 4] = [
        Buffer::from_shorts(&ATTACK_BYTES[0..32], BrightWhite, None),
        Buffer::from_shorts(&ATTACK_BYTES[32..64], BrightWhite, None),
        Buffer::from_shorts(&ATTACK_BYTES[64..96], BrightWhite, None),
        Buffer::from_shorts(&ATTACK_BYTES[96..], BrightWhite, None),
    ];
    pub static ref DRAGON_BURN_FX: [Buffer; 9] = [
        Buffer::from_shorts(&DRAGON_BURN_BYTES[0..32], BrightYellow, None),
        Buffer::from_shorts(&DRAGON_BURN_BYTES[32..64], BrightYellow, None),
        Buffer::from_shorts(&DRAGON_BURN_BYTES[64..96], BrightYellow, None),
        Buffer::from_shorts(&DRAGON_BURN_BYTES[96..128], BrightYellow, None),
        Buffer::from_shorts(&DRAGON_BURN_BYTES[128..160], BrightYellow, None),
        Buffer::from_shorts(&DRAGON_BURN_BYTES[160..192], BrightYellow, None),
        Buffer::from_shorts(&DRAGON_BURN_BYTES[192..224], BrightYellow, None),
        Buffer::from_shorts(&DRAGON_BURN_BYTES[224..256], BrightYellow, None),
        Buffer::from_shorts(&DRAGON_BURN_BYTES[256..], BrightYellow, None),
    ];
    pub static ref EXPLODING_CIRCLE_FX: [Buffer; 9] = [
        Buffer::from_shorts(&EXPLODING_CIRCLE_BYTES[0..32], BrightWhite, None),
        Buffer::from_shorts(&EXPLODING_CIRCLE_BYTES[32..64], BrightWhite, None),
        Buffer::from_shorts(&EXPLODING_CIRCLE_BYTES[64..96], BrightWhite, None),
        Buffer::from_shorts(&EXPLODING_CIRCLE_BYTES[96..128], BrightWhite, None),
        Buffer::from_shorts(&EXPLODING_CIRCLE_BYTES[128..160], BrightWhite, None),
        Buffer::from_shorts(&EXPLODING_CIRCLE_BYTES[160..192], BrightWhite, None),
        Buffer::from_shorts(&EXPLODING_CIRCLE_BYTES[192..224], BrightWhite, None),
        Buffer::from_shorts(&EXPLODING_CIRCLE_BYTES[224..256], BrightWhite, None),
        Buffer::from_shorts(&EXPLODING_CIRCLE_BYTES[256..], BrightWhite, None),
    ];
    pub static ref EXPLOSION_FX: [Buffer; 7] = [
        Buffer::from_shorts(&EXPLOSION_BYTES[0..32], BrightYellow, None),
        Buffer::from_shorts(&EXPLOSION_BYTES[32..64], BrightYellow, None),
        Buffer::from_shorts(&EXPLOSION_BYTES[64..96], BrightYellow, None),
        Buffer::from_shorts(&EXPLOSION_BYTES[96..128], BrightYellow, None),
        Buffer::from_shorts(&EXPLOSION_BYTES[128..160], BrightYellow, None),
        Buffer::from_shorts(&EXPLOSION_BYTES[160..192], BrightYellow, None),
        Buffer::from_shorts(&EXPLOSION_BYTES[192..], BrightYellow, None),
    ];
    pub static ref TWIRL_FX: [Buffer; 10] = [
        Buffer::from_shorts(&TWIRL_BYTES[0..32], BrightCyan, None),
        Buffer::from_shorts(&TWIRL_BYTES[32..64], BrightCyan, None),
        Buffer::from_shorts(&TWIRL_BYTES[64..96], BrightCyan, None),
        Buffer::from_shorts(&TWIRL_BYTES[96..128], BrightCyan, None),
        Buffer::from_shorts(&TWIRL_BYTES[128..160], BrightCyan, None),
        Buffer::from_shorts(&TWIRL_BYTES[160..192], BrightCyan, None),
        Buffer::from_shorts(&TWIRL_BYTES[192..224], BrightCyan, None),
        Buffer::from_shorts(&TWIRL_BYTES[224..256], BrightCyan, None),
        Buffer::from_shorts(&TWIRL_BYTES[256..288], BrightCyan, None),
        Buffer::from_shorts(&TWIRL_BYTES[288..], BrightCyan, None),
    ];
}
