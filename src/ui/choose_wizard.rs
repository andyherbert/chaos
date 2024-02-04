use crate::config::Player;
use crate::data::wizard::{WizardCharacter, WizardColor};
use crate::error::ChaosError;
use crate::gfx::color::Color::*;
use crate::window::Window;

pub fn choose_wizard(win: &mut Window, player: &Option<Player>) -> Result<Option<Player>, ChaosError> {
    win.buf.clear();
    win.buf.screen_border("CHOOSE YOUR WIZARD", BrightBlue, BrightCyan);
    win.buf.draw_text("PLAYER", 36, 4, BrightYellow);
    win.buf.draw_text("Enter name (12 letters max.)", 36, 6, BrightMagenta);
    let name = match player {
        Some(player) => player.name.clone(),
        None => String::new(),
    };
    let name = match win.wizard_name(name, 36, 8, 12, BrightCyan)? {
        Some(name) => name,
        None => return Ok(None),
    };
    win.buf.draw_text("Which character?", 36, 10, BrightMagenta);
    win.buf.draw_text("1  2  3  4  5  6  7  8", 36, 12, BrightCyan);
    for index in 0..8 {
        let wizard: WizardCharacter = index.try_into()?;
        let buf = wizard.as_buffer(WizardColor::BrightCyan);
        win.buf.draw_buffer(&buf, 37 + (index as usize * 3), 12);
    }
    let character_num = match win.wait_for_number(1..=8)? {
        Some(character_num) => {
            let text = format!("{}", character_num);
            win.buf.draw_text(&text, 53, 10, BrightWhite);
            character_num - 1
        }
        None => return Ok(None),
    };
    let character = WizardCharacter::try_from(character_num)?;
    let buf = character.as_buffer(WizardColor::BrightWhite);
    win.buf.draw_buffer(&buf, 54, 10);
    win.buf.draw_text("Which colour?", 36, 14, BrightMagenta);
    win.buf.draw_text("1  2  3  4  5  6  7  8", 36, 16, BrightYellow);
    for index in 0..8 {
        let color: WizardColor = index.try_into()?;
        let buf = character.as_buffer(color);
        win.buf.draw_buffer(&buf, 37 + (index as usize * 3), 16);
    }
    let color_num = match win.wait_for_number(1..=8)? {
        Some(color_num) => {
            let text = format!("{}", color_num);
            win.buf.draw_text(&text, 50, 14, BrightWhite);
            color_num - 1
        }
        None => return Ok(None),
    };
    let color = color_num.try_into()?;
    let buf = character.as_buffer(color);
    win.buf.draw_buffer(&buf, 51, 14);
    win.wait(900)?;
    let player_config = Player { name, character, color };
    Ok(Some(player_config))
}
