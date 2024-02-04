pub mod chaos_server;
mod game_logic;
mod sender;
mod server_state;
use super::{MessageReader, MessageWriter, NetworkError, RecieveMsg, SendMsg, ServerMessage};
use crate::config::NetAddress;
use tokio::net::{TcpListener, TcpStream};
use tokio::select;
use tokio::sync::{broadcast, mpsc};
use tokio::time::{interval, Duration};

pub async fn connection_loop(
    mut stream: TcpStream,
    tx: mpsc::Sender<RecieveMsg>,
    mut rx: broadcast::Receiver<SendMsg>,
    id: u32,
) -> Result<(), NetworkError> {
    let (mut reader, mut writer) = stream.split();
    tx.send(RecieveMsg::Connected { id }).await?;
    let mut reader = MessageReader::new(&mut reader);
    let mut writer = MessageWriter::new(&mut writer);
    let mut interval = interval(Duration::from_secs(5));
    loop {
        select! {
            read = reader.read() => {
                match read {
                    Err(_) => {
                        tx.send(RecieveMsg::Disconnected { id }).await?;
                        return Ok(());
                    }
                    Ok(ServerMessage::ClientMessage {
                        msg,
                    }) => {
                        tx.send(RecieveMsg::Message { id, msg }).await?;
                    }
                    Ok(ServerMessage::Ping(time)) => {
                        writer.pong(time).await?;
                    }
                    Ok(ServerMessage::Pong(delta)) => {
                        tx.send(RecieveMsg::Latency { id, delta }).await?;
                    }
                    _ => unreachable!(),
                }
            }
            rx = rx.recv() => {
                let rx = rx?;
                match rx {
                    SendMsg::MessageToAll { id: msg_id, msg } => {
                        writer.write(ServerMessage::OutgoingMessage {
                            id: msg_id.unwrap_or(id),
                            msg,
                        }).await?;
                    }
                    SendMsg::MessageToId { to, id: from, msg } => {
                        if to == id {
                            writer.write(ServerMessage::OutgoingMessage {
                                id: from,
                                msg,
                            }).await?;
                        }
                    }
                    SendMsg::MessageToAllExcept { id: from, msg } => {
                        if from != id {
                            writer.write(ServerMessage::OutgoingMessage {
                                id: from,
                                msg,
                            }).await?;
                        }
                    }
                    SendMsg::Shutdown => {
                        writer.shutdown().await?;
                        return Ok(());
                    }
                }
            }
            _ = interval.tick() => {
                writer.ping().await?;
            }
        }
    }
}

async fn server_loop(
    listener: TcpListener,
    conn_tx: mpsc::Sender<RecieveMsg>,
    broad_tx: broadcast::Sender<SendMsg>,
    mut rx: mpsc::Receiver<SendMsg>,
) -> Result<(), NetworkError> {
    let mut id = 0;
    loop {
        select! {
            Ok((stream, _addr)) = listener.accept() => {
                tokio::spawn(connection_loop(stream, conn_tx.clone(), broad_tx.subscribe(), id));
                id += 1;
            }
            Some(msg) = rx.recv() => {
                if let SendMsg::Shutdown = msg {
                    broad_tx.send(msg)?;
                    return Ok(());
                }
                broad_tx.send(msg)?;
            }
        }
    }
}

pub async fn spawn_server(addr: &NetAddress) -> Result<(mpsc::Sender<SendMsg>, mpsc::Receiver<RecieveMsg>), NetworkError> {
    let (tx, rx) = mpsc::channel(64);
    let (conn_tx, conn_rx) = mpsc::channel(64);
    let (broad_tx, _broad_rx) = broadcast::channel(64);
    let addr = format!("{}:{}", addr.host, addr.port);
    let listener = TcpListener::bind(addr).await?;
    tokio::spawn(server_loop(listener, conn_tx, broad_tx, rx));
    Ok((tx, conn_rx))
}
