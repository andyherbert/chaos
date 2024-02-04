use crate::data::arena::ArenaError;
use crate::data::wizard::WizardError;
use crate::net::NetworkError;
use std::net::AddrParseError;
use std::sync::mpsc::TryRecvError;
use std::{error, fmt, io};
use toml::de;

#[derive(Debug)]
pub enum ChaosError {
    GameError,
    IOError,
    NetworkError,
    Quit,
}

impl From<minifb::Error> for ChaosError {
    fn from(_value: minifb::Error) -> Self {
        Self::GameError
    }
}

impl From<WizardError> for ChaosError {
    fn from(_value: WizardError) -> Self {
        Self::GameError
    }
}

impl From<io::Error> for ChaosError {
    fn from(_value: io::Error) -> Self {
        Self::IOError
    }
}

impl From<de::Error> for ChaosError {
    fn from(_value: de::Error) -> Self {
        Self::IOError
    }
}

impl From<toml::ser::Error> for ChaosError {
    fn from(_value: toml::ser::Error) -> Self {
        Self::IOError
    }
}

impl From<NetworkError> for ChaosError {
    fn from(_value: NetworkError) -> Self {
        Self::NetworkError
    }
}

impl From<AddrParseError> for ChaosError {
    fn from(_value: AddrParseError) -> Self {
        Self::GameError
    }
}

impl From<TryRecvError> for ChaosError {
    fn from(_value: TryRecvError) -> Self {
        Self::GameError
    }
}

impl From<ArenaError> for ChaosError {
    fn from(_value: ArenaError) -> Self {
        Self::GameError
    }
}

impl fmt::Display for ChaosError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ChaosError::GameError => write!(f, "Application error"),
            ChaosError::IOError => write!(f, "I/O error"),
            ChaosError::NetworkError => write!(f, "Network error"),
            ChaosError::Quit => write!(f, "Quit"),
        }
    }
}

impl error::Error for ChaosError {}
