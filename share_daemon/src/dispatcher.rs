use std::net::{SocketAddr, TcpStream};

use crossbeam::channel::{Receiver, Sender};

use crate::handler::{Data, Handler};

pub struct Dispatcher {
    handler_tx: Sender<Handler>,
    data_rx: Receiver<Data>,
}

impl Dispatcher {
    pub(crate) fn new(handler_tx: Sender<Handler>, data_rx: Receiver<Data>) -> Self {
        Self {
            handler_tx,
            data_rx,
        }
    }
    pub(crate) fn dispatch(&self, stream: TcpStream, remote_addr: SocketAddr) {
        // self.handler_tx.send(msg)
        // self.data_rx.recv().unwrap()
    }
}
