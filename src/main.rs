mod config;
mod data;
mod error;
mod gfx;
mod net;
mod ui;
mod window;
use clap::Parser;
use config::{GameConfig, NetAddress, Player};
use data::wizard::{WizardCharacter, WizardColor};
use error::ChaosError;
use gfx::buffer::{Buffer, LOGO, SNAKE};
use gfx::color::Color::*;
use net::{ChaosClient, ChaosServer};
use ui::{choose_wizard, host_game, join_game, lobby};
use window::Window;

#[derive(Parser)]
struct Cli {
    #[clap(short = '1', conflicts_with = "debug_2")]
    debug_1: bool,
    #[clap(short = '2')]
    debug_2: bool,
}

async fn start_game(
    win: &mut Window,
    player: Player,
    host_addr: Option<&NetAddress>,
    addr: &NetAddress,
) -> Result<(), ChaosError> {
    let server = match host_addr {
        Some(host) => Some(ChaosServer::new(host).await?),
        None => None,
    };
    let mut client = ChaosClient::new(addr).await?;
    if let Err(err) = lobby(win, player, &mut client).await {
        client.disconnect().ok();
        if let Some(server) = server {
            server.shutdown()?;
        }
        return Err(err);
    }
    client.disconnect().ok();
    if let Some(server) = server {
        server.shutdown()?;
    }
    Ok(())
}

fn use_or_obtain_player(win: &mut Window, config: &mut GameConfig) -> Result<Option<Player>, ChaosError> {
    match config.player {
        None => match choose_wizard(win, &config.player)? {
            Some(player) => {
                config.player = Some(player.clone());
                config.save()?;
                Ok(Some(player))
            }
            None => Ok(None),
        },
        Some(ref player) => Ok(Some(player.clone())),
    }
}

fn about_screen(win: &mut Window) -> Result<(), ChaosError> {
    win.buf.clear();
    win.buf.screen_border("PRESS ANY KEY", BrightBlue, BrightCyan);
    win.buf.draw_text(include_str!("txt/about.txt"), 2, 2, BrightWhite);
    win.wait_for_any_key()
}

fn error_screen(win: &mut Window, err: ChaosError) -> Result<(), ChaosError> {
    win.buf.clear();
    win.buf.screen_border("PRESS ANY KEY TO CONTINUE", BrightRed, BrightYellow);
    win.buf.center_text(&err.to_string(), 10, White);
    win.wait_for_any_key()?;
    Ok(())
}

async fn main_menu(win: &mut Window) -> Result<(), ChaosError> {
    let mut config = GameConfig::load()?;
    loop {
        win.buf.clear();
        if let Some(ref player) = config.player {
            let buf = Buffer::from(player);
            win.buf.draw_buffer(&buf, 40 + player.name.len(), 3);
            win.buf.draw_text(&player.name, 40, 3, BrightYellow);
        }
        win.buf.screen_border("PRESS KEYS 1 TO 5", BrightRed, BrightYellow);
        win.buf.draw_text("1.CHANGE WIZARD", 40, 7, BrightCyan);
        win.buf.draw_text("2.HOST GAME", 40, 9, BrightCyan);
        win.buf.draw_text("3.JOIN GAME", 40, 11, BrightCyan);
        win.buf.draw_text("4.ABOUT CHAOS", 40, 13, BrightCyan);
        win.buf.draw_text("5.QUIT", 40, 15, BrightCyan);
        match win.wait_for_number(1..=5)? {
            Some(1) => {
                if let Some(player_config) = choose_wizard(win, &config.player)? {
                    config.player = Some(player_config);
                    config.save()?;
                }
            }
            Some(2) => {
                if let Some(player) = use_or_obtain_player(win, &mut config)? {
                    if let Some(addr) = host_game(win, &config.last_host)? {
                        config.last_host = Some(addr.clone());
                        config.save()?;
                        start_game(win, player, Some(&addr), &addr).await?;
                    }
                }
            }
            Some(3) => {
                if let Some(player) = use_or_obtain_player(win, &mut config)? {
                    if let Some(addr) = join_game(win, &config.last_host)? {
                        config.last_host = Some(addr.clone());
                        config.save()?;
                        start_game(win, player, None, &addr).await?;
                    }
                }
            }
            Some(4) => about_screen(win)?,
            Some(5) | None => win.quit()?,
            _ => unreachable!("Invalid menu option"),
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), ChaosError> {
    let args = Cli::parse();
    let win = &mut Window::new()?;
    if args.debug_1 {
        let player = Player {
            name: "Gandalf".to_string(),
            character: WizardCharacter::AsimonoZark,
            color: WizardColor::BrightWhite,
        };
        let addr = NetAddress::default();
        start_game(win, player, Some(&addr), &addr).await?;
    } else if args.debug_2 {
        let player = Player {
            name: "Julian".to_string(),
            character: WizardCharacter::Dyerarti,
            color: WizardColor::BrightYellow,
        };
        let addr = NetAddress::default();
        start_game(win, player, None, &addr).await?;
    } else {
        win.buf.clear();
        win.buf.draw_buffer(&LOGO, 39, 2);
        win.buf.center_text("THE BATTLE OF WIZARDS", 7, BrightMagenta);
        win.buf.center_text("© Games Workshop 1985", 11, BrightYellow);
        win.buf.center_text("By Julian Gollop", 13, BrightRed);
        win.buf.center_text("*Raised From The Dead", 16, BrightGreen);
        win.buf.center_text("By Andrew Herbert", 18, BrightRed);
        win.buf.draw_text(env!("CARGO_PKG_VERSION"), 0, 22, BrightRed);
        win.buf.draw_buffer(&SNAKE, 64, 9);
        win.wait_for_any_key()?;
        loop {
            if let Err(err) = main_menu(win).await {
                if let ChaosError::Quit = err {
                    break;
                } else {
                    error_screen(win, err)?;
                }
            }
        }
    }
    Ok(())
}
