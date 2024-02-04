use super::{ClientMessage, RecieveMsg, SendMsg};
use crate::data::arena::ArenaError;
use std::net::AddrParseError;
use std::time::SystemTimeError;
use std::{error, fmt, io};
use tokio::sync::{broadcast, mpsc};

#[derive(Debug)]
pub enum NetworkError {
    GenericError,
    Shutdown,
    Disconnected,
}

impl From<AddrParseError> for NetworkError {
    fn from(_err: AddrParseError) -> Self {
        NetworkError::GenericError
    }
}

impl From<mpsc::error::SendError<RecieveMsg>> for NetworkError {
    fn from(_err: mpsc::error::SendError<RecieveMsg>) -> Self {
        NetworkError::GenericError
    }
}

impl From<mpsc::error::SendError<SendMsg>> for NetworkError {
    fn from(_err: mpsc::error::SendError<SendMsg>) -> Self {
        NetworkError::GenericError
    }
}

impl From<broadcast::error::SendError<SendMsg>> for NetworkError {
    fn from(_err: broadcast::error::SendError<SendMsg>) -> Self {
        NetworkError::GenericError
    }
}

impl From<broadcast::error::RecvError> for NetworkError {
    fn from(_err: broadcast::error::RecvError) -> Self {
        NetworkError::GenericError
    }
}

impl From<mpsc::error::SendError<ClientMessage>> for NetworkError {
    fn from(_err: mpsc::error::SendError<ClientMessage>) -> Self {
        NetworkError::GenericError
    }
}

impl From<bincode::Error> for NetworkError {
    fn from(_err: bincode::Error) -> Self {
        NetworkError::GenericError
    }
}

impl From<io::Error> for NetworkError {
    fn from(_err: io::Error) -> Self {
        NetworkError::GenericError
    }
}

impl From<SystemTimeError> for NetworkError {
    fn from(_err: SystemTimeError) -> Self {
        NetworkError::GenericError
    }
}

impl From<mpsc::error::TryRecvError> for NetworkError {
    fn from(_err: mpsc::error::TryRecvError) -> Self {
        NetworkError::GenericError
    }
}

impl From<mpsc::error::TrySendError<ClientMessage>> for NetworkError {
    fn from(_err: mpsc::error::TrySendError<ClientMessage>) -> Self {
        NetworkError::GenericError
    }
}

impl From<ArenaError> for NetworkError {
    fn from(_err: ArenaError) -> Self {
        NetworkError::GenericError
    }
}

impl error::Error for NetworkError {}

impl fmt::Display for NetworkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        use NetworkError::*;
        match self {
            GenericError => write!(f, "Network error"),
            Shutdown => write!(f, "Shutdown"),
            Disconnected => write!(f, "Disconnected"),
        }
    }
}
