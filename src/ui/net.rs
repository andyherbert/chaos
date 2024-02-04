use crate::config::NetAddress;
use crate::error::ChaosError;
use crate::gfx::color::Color::*;
use crate::window::Window;

fn network(win: &mut Window, title: &str, address: NetAddress) -> Result<Option<NetAddress>, ChaosError> {
    win.buf.clear();
    win.buf.screen_border(title, BrightBlue, BrightCyan);
    win.buf.draw_text("SERVER", 38, 6, BrightYellow);
    win.buf.draw_text("Host", 38, 8, BrightMagenta);
    let host = match win.host_entry(address.host, 38, 10, 52, BrightCyan)? {
        Some(host) => host,
        None => return Ok(None),
    };
    win.buf.draw_text("Port", 38, 12, BrightMagenta);
    let port = match win.port_entry(address.port, 38, 14, 5, BrightCyan)? {
        Some(port) => port,
        None => return Ok(None),
    };
    win.wait(900)?;
    Ok(Some(NetAddress { host, port }))
}

pub fn host_game(win: &mut Window, address: &Option<NetAddress>) -> Result<Option<NetAddress>, ChaosError> {
    let addr = address.clone().unwrap_or_default();
    network(win, "HOST GAME", addr)
}

pub fn join_game(win: &mut Window, address: &Option<NetAddress>) -> Result<Option<NetAddress>, ChaosError> {
    let addr = address.clone().unwrap_or_default();
    network(win, "JOIN GAME", addr)
}
