use super::game_logic::GameLogic;
use crate::config::NetAddress;
use crate::error::ChaosError;
use crate::net::{server::spawn_server, NetworkError};
use tokio::sync::oneshot::{self, Sender};

pub struct ChaosServer {
    quit_tx: Sender<()>,
}

impl ChaosServer {
    pub async fn new(addr: &NetAddress) -> Result<Self, NetworkError> {
        let (quit_tx, quit_rx) = oneshot::channel();
        let (tx, rx) = spawn_server(addr).await?;
        tokio::spawn(async move {
            let mut game = GameLogic::new(rx, tx, quit_rx);
            if let Some(wizards) = game.lobby_loop().await? {
                let winners = game.game_loop(wizards).await?;
                game.end(winners).await?;
            }
            Ok::<(), ChaosError>(())
        });
        Ok(Self { quit_tx })
    }

    pub fn shutdown(self) -> Result<(), NetworkError> {
        self.quit_tx.send(()).map_err(|_| NetworkError::Shutdown)
    }
}
