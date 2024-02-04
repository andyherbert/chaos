use super::{ClientMessage, Message, MessageReader, MessageWriter, NetworkError, ServerMessage};
use crate::config::NetAddress;
use tokio::net::TcpStream;
use tokio::select;
use tokio::sync::mpsc::{self, error::TryRecvError};
use tokio::time::{interval, Duration};

async fn client_loop(
    mut stream: TcpStream,
    tx: mpsc::Sender<ClientMessage>,
    mut rx: mpsc::Receiver<ClientMessage>,
) -> Result<(), NetworkError> {
    let (mut reader, mut writer) = stream.split();
    let mut reader = MessageReader::new(&mut reader);
    let mut writer = MessageWriter::new(&mut writer);
    let mut interval = interval(Duration::from_secs(5));
    loop {
        select! {
            read = reader.read() => {
                match read {
                    Err(_) => {
                        tx.send(ClientMessage::Disconnect).await?;
                        return Ok(());
                    }
                    Ok(ServerMessage::OutgoingMessage {
                        id,
                        msg,
                    }) => {
                        tx.send(ClientMessage::IncomingMessage {
                            id,
                            msg,
                        }).await?;
                    }
                    Ok(ServerMessage::Ping(time)) => {
                        writer.pong(time).await?;
                    }
                    Ok(ServerMessage::Pong(delta)) => {
                        tx.send(ClientMessage::Latency(delta)).await?;
                    }
                    _ => unreachable!(),
                }
            }
            Some(msg) = rx.recv() => {
                match msg {
                    ClientMessage::OutgoingMessage {
                        msg,
                    } => {
                        writer.write(ServerMessage::ClientMessage {
                            msg,
                        }).await?;
                    }
                    ClientMessage::Disconnect => {
                        writer.shutdown().await?;
                    }
                    _ => {}
                }
            }
            _ = interval.tick() => {
                writer.ping().await?;
            }
        }
    }
}

pub struct ChaosClient {
    tx: mpsc::Sender<ClientMessage>,
    rx: mpsc::Receiver<ClientMessage>,
}

impl ChaosClient {
    pub async fn new(addr: &NetAddress) -> Result<Self, NetworkError> {
        let addr = format!("{}:{}", addr.host, addr.port);
        let stream = TcpStream::connect(addr).await?;
        let (conn_tx, conn_rx) = mpsc::channel(64);
        let (send_tx, send_rx) = mpsc::channel(64);
        tokio::spawn(client_loop(stream, conn_tx, send_rx));
        Ok(Self {
            tx: send_tx,
            rx: conn_rx,
        })
    }

    pub fn send(&mut self, msg: Message) -> Result<(), NetworkError> {
        self.tx.try_send(ClientMessage::OutgoingMessage { msg })?;
        Ok(())
    }

    pub fn recv(&mut self) -> Result<Option<(u32, Message)>, NetworkError> {
        match self.rx.try_recv() {
            Ok(ClientMessage::IncomingMessage { msg, id }) => Ok(Some((id, msg))),
            Ok(ClientMessage::Disconnect) => Err(NetworkError::Disconnected),
            Ok(ClientMessage::Latency(_)) => Ok(None),
            Err(TryRecvError::Empty) => Ok(None),
            Err(_) => Err(NetworkError::GenericError),
            _ => unreachable!("unexpected message"),
        }
    }

    pub fn disconnect(self) -> Result<(), NetworkError> {
        self.tx.try_send(ClientMessage::Disconnect)?;
        Ok(())
    }
}
