use std::{collections::HashMap, net::SocketAddr, sync::Arc};

use fastwebsockets::Frame;
use tokio::sync::{mpsc::UnboundedSender, RwLock};

pub type Tx = UnboundedSender<WsMessage>;
pub type SharedState = Arc<RwLock<State>>;

pub struct State {
    pub clients: HashMap<SocketAddr, Tx>,
}

impl State {
    pub async fn broadcast(&self, sender: &SocketAddr, msg: WsMessage) {
        for (addr, tx) in self.clients.iter() {
            if addr != sender {
                tx.send(msg.clone()).unwrap();
            }
        }
    }
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub enum WsMessage {
    Text(String),
    Binary(Vec<u8>),

    /// Send a pong message with the given data.
    Pong(Vec<u8>),

    /// Close the connection with the given code and reason.
    ///
    /// u16 is the status code
    /// String is the reason
    Close(u16, String),
}

impl WsMessage {
    pub fn to_frame(&self) -> Frame {
        match self {
            WsMessage::Text(text) => Frame::text(text.as_bytes().into()),
            WsMessage::Binary(data) => Frame::binary(data.as_slice().into()),
            WsMessage::Pong(data) => Frame::pong(data.as_slice().into()),
            WsMessage::Close(code, reason) => Frame::close(*code, reason.as_bytes()),
        }
    }
}
