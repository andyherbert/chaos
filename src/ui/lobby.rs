use super::game::game;
use crate::config::Player;
use crate::data::wizard::{LobbyWizard, LobbyWizards};
use crate::error::ChaosError;
use crate::gfx::color::Color::*;
use crate::net::{ChaosClient, Message};
use crate::window::{Key, Window};

fn lobby_list(win: &mut Window, wizards: impl Iterator<Item = LobbyWizard>) -> Result<(), ChaosError> {
    win.buf.clear_area(42, 4, 14, 16);
    for (i, wizard) in wizards.enumerate() {
        let color = if wizard.ready { BrightYellow } else { Red };
        win.buf.center_player(&wizard.player, 4 + 2 * i, color, wizard.ready);
    }
    Ok(())
}

pub async fn lobby(win: &mut Window, player: Player, client: &mut ChaosClient) -> Result<(), ChaosError> {
    let mut wizards = LobbyWizards::new();
    win.buf.clear();
    win.buf.screen_border("ARE YOU READY? (Y OR N)", BrightRed, BrightYellow);
    win.buf
        .center_text("THE GAME WILL START WHEN ALL WIZARDS ARE READY", 2, BrightMagenta);
    client.send(Message::Join(player.clone()))?;
    loop {
        win.update()?;
        match win.get_yes_or_no_or_cancel() {
            Some(Key::Y) => client.send(Message::Ready(true))?,
            Some(Key::N) => client.send(Message::Ready(false))?,
            Some(Key::Escape) => return Ok(()),
            _ => {}
        }
        if let Some(msg) = client.recv()? {
            match msg {
                (id, Message::Join(player)) => {
                    if wizards.join(id, player) {
                        lobby_list(win, wizards.players())?;
                    }
                }
                (id, Message::Leave(_)) => {
                    if wizards.leave(id).is_some() {
                        lobby_list(win, wizards.players())?;
                    }
                }
                (id, Message::Ready(ready)) => {
                    if wizards.ready(id, ready) {
                        lobby_list(win, wizards.players())?;
                    }
                }
                (_, Message::Start(wizard)) => {
                    game(win, client, wizard)?;
                    return Ok(());
                }
                _ => {}
            }
        }
    }
}
