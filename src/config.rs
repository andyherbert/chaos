use crate::data::stats::Frame;
use crate::data::wizard::{WizardCharacter, WizardColor};
use crate::error::ChaosError;
use crate::gfx::buffer::Buffer;
use directories::BaseDirs;
use serde::{Deserialize, Serialize};
use std::fs::{create_dir_all, read_to_string, File};
use std::io::Write;
use std::path::Path;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Player {
    pub name: String,
    pub character: WizardCharacter,
    pub color: WizardColor,
}

impl From<&Player> for Buffer {
    fn from(player: &Player) -> Self {
        player.character.as_buffer(player.color)
    }
}

impl From<&Player> for Frame {
    fn from(player: &Player) -> Self {
        Frame {
            bytes: player.character.as_bytes().try_into().expect("Invalid character"),
            fg: player.color.into(),
            bg: None,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NetAddress {
    pub host: String,
    pub port: usize,
}

impl Default for NetAddress {
    fn default() -> Self {
        Self {
            host: "localhost".to_string(),
            port: 8080,
        }
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct GameConfig {
    pub player: Option<Player>,
    pub last_host: Option<NetAddress>,
    pub last_join: Option<NetAddress>,
}

impl GameConfig {
    pub fn load() -> Result<GameConfig, ChaosError> {
        if let Some(base) = BaseDirs::new() {
            let path = Path::new(base.config_dir()).join("Chaos").join("Config.toml");
            if path.exists() {
                let string = read_to_string(path)?;
                let config = toml::from_str(&string)?;
                return Ok(config);
            }
        }
        Ok(GameConfig::default())
    }

    pub fn save(&self) -> Result<(), ChaosError> {
        let string = toml::to_string_pretty(&self)?;
        if let Some(base) = BaseDirs::new() {
            let path = Path::new(base.config_dir()).join("Chaos");
            if !path.exists() {
                create_dir_all(&path)?;
            }
            let path = path.join("Config.toml");
            let mut file = File::create(path)?;
            file.write_all(string.as_bytes())?;
        }
        Ok(())
    }
}
