mod client;
mod error;
mod server;
use crate::config::Player;
use crate::data::creation::GameCreation;
use crate::data::spells::Spell;
use crate::data::stats::WizardStats;
use crate::data::wizard::{GameWizard, Wizard};
use crate::gfx::color::Color;
pub use client::ChaosClient;
pub use error::NetworkError;
use serde::{Deserialize, Serialize};
pub use server::chaos_server::ChaosServer;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader, BufWriter};
use tokio::net::tcp::{ReadHalf, WriteHalf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServerMessage {
    OutgoingMessage { id: u32, msg: Message },
    ClientMessage { msg: Message },
    Ping(u128),
    Pong(u128),
}

pub enum ClientMessage {
    OutgoingMessage { msg: Message },
    IncomingMessage { id: u32, msg: Message },
    Disconnect,
    Latency(u128),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Message {
    Join(Player),
    Leave(u32),
    Ready(bool),
    Start(Wizard),
    AddWizard {
        wizard: GameWizard,
        x: u8,
        y: u8,
    },
    Disbelieve {
        x: u8,
        y: u8,
        success: bool,
    },
    CreationSpell {
        x: u8,
        y: u8,
        creation: Option<GameCreation>,
    },
    CastFire {
        x: u8,
        y: u8,
        fire: Option<GameCreation>,
    },
    CastBlob {
        x: u8,
        y: u8,
        blob: Option<GameCreation>,
    },
    SendSpell {
        spell: Spell,
    },
    NewSpell {
        x: u8,
        y: u8,
    },
    ShadowWoodInfo,
    NoPossibleMoves,
    BuffWizard(WizardStats),
    DeBuffWizard(WizardStats),
    ChooseSpell,
    ChosenSpell(Option<(u32, bool)>),
    WaitingForOtherPlayers(u32),
    CastSpell {
        spell_name: String,
        range: u8,
    },
    MovementRange {
        range: u8,
        flying: bool,
        tiles: Vec<(u8, u8)>,
    },
    MovementPoints {
        points: u8,
        tiles: Vec<(u8, u8)>,
    },
    UndeadCannotBeAttacked,
    FailedAttack {
        x: u8,
        y: u8,
    },
    SuccessfulAttack {
        x: u8,
        y: u8,
        corpse: bool,
    },
    FailedRangedAttack {
        sx: u8,
        sy: u8,
        dx: u8,
        dy: u8,
        color: Color,
    },
    SuccessfulRangedAttack {
        sx: u8,
        sy: u8,
        dx: u8,
        dy: u8,
        corpse: bool,
        color: Color,
    },
    FailedDragonRangedAttack {
        sx: u8,
        sy: u8,
        dx: u8,
        dy: u8,
    },
    SuccessfulDragonRangedAttack {
        sx: u8,
        sy: u8,
        dx: u8,
        dy: u8,
    },
    Subversion {
        x: u8,
        y: u8,
        success: bool,
    },
    RaiseDead {
        x: u8,
        y: u8,
        success: bool,
    },
    MagicBolt {
        x: u8,
        y: u8,
        success: bool,
    },
    Lightning {
        x: u8,
        y: u8,
        success: bool,
    },
    ShelterDisappears {
        x: u8,
        y: u8,
    },
    MagicalAttack {
        x: u8,
        y: u8,
        success: bool,
    },
    SpawnFire {
        x: u8,
        y: u8,
        fire: Option<GameCreation>,
    },
    SpawnBlob {
        x: u8,
        y: u8,
        blob: Option<GameCreation>,
    },
    RemoveSpawn {
        x: u8,
        y: u8,
    },
    NoLineOfSight,
    ChoosePiece(Vec<(u8, u8)>),
    ChooseTarget(Vec<(u8, u8)>),
    ChooseCombat(Vec<(u8, u8)>),
    EngagedInCombat(Vec<(u8, u8)>),
    ChooseRangedCombat {
        range: u8,
        tiles: Vec<(u8, u8)>,
    },
    ChosenTile(Option<u8>),
    SpellSucceeds(i8),
    SpellFails,
    Turn,
    TurnEnd,
    MoveWizard {
        x: u8,
        y: u8,
    },
    MoveCreation {
        sx: u8,
        sy: u8,
        dx: u8,
        dy: u8,
    },
    AskForDismount,
    Dismount(Option<bool>),
    Results(Vec<Player>),
    Shutdown,
}

#[derive(Debug, Clone)]
pub enum SendMsg {
    MessageToAll { id: Option<u32>, msg: Message },
    MessageToId { to: u32, id: u32, msg: Message },
    MessageToAllExcept { id: u32, msg: Message },
    Shutdown,
}

#[allow(clippy::large_enum_variant)]
pub enum RecieveMsg {
    Connected { id: u32 },
    Disconnected { id: u32 },
    Message { id: u32, msg: Message },
    Latency { id: u32, delta: u128 },
}

pub struct MessageReader<'a> {
    reader: BufReader<&'a mut ReadHalf<'a>>,
}

impl<'a> MessageReader<'a> {
    pub fn new(reader: &'a mut ReadHalf<'a>) -> Self {
        let reader = BufReader::new(reader);
        Self { reader }
    }

    pub async fn read(&mut self) -> Result<ServerMessage, NetworkError> {
        let len = self.reader.read_u32().await?;
        let mut buf = vec![0; len as usize];
        self.reader.read_exact(&mut buf).await?;
        let msg = bincode::deserialize(&buf)?;
        Ok(msg)
    }
}

pub struct MessageWriter<'a> {
    writer: BufWriter<&'a mut WriteHalf<'a>>,
}

impl<'a> MessageWriter<'a> {
    pub fn new(writer: &'a mut WriteHalf<'a>) -> Self {
        let writer = BufWriter::new(writer);
        Self { writer }
    }

    pub async fn write(&mut self, msg: ServerMessage) -> Result<(), NetworkError> {
        let buf = bincode::serialize(&msg)?;
        self.writer.write_u32(buf.len() as u32).await?;
        self.writer.write_all(&buf).await?;
        self.writer.flush().await?;
        Ok(())
    }

    pub async fn shutdown(&mut self) -> Result<(), NetworkError> {
        self.writer.shutdown().await?;
        Ok(())
    }

    pub async fn ping(&mut self) -> Result<(), NetworkError> {
        let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis();
        self.write(ServerMessage::Ping(now)).await?;
        Ok(())
    }

    pub async fn pong(&mut self, time: u128) -> Result<(), NetworkError> {
        let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis();
        let delta = now - time;
        self.write(ServerMessage::Pong(delta)).await?;
        Ok(())
    }
}
