use std::{
    collections::VecDeque,
    io::BufWriter,
    net::{SocketAddr, TcpStream},
    path::PathBuf,
    thread::{self, JoinHandle},
};

use crossbeam::channel::{Receiver, Sender};

use crate::handler::{Data, Handler};

#[derive(Debug)]
pub struct Workers {
    data_sender: Sender<Data>,
    _inner_handles: Vec<JoinHandle<()>>,
}

impl Workers {
    pub(crate) fn start(rx: Receiver<Handler>, data_tx: Sender<Data>, num_workers: u8) -> Self {
        let mut index = 0;
        let mut handles = vec![];
        while index <= num_workers as usize {
            let rx_in = rx.clone();
            handles[index] = thread::spawn(move || Self::work_loop(rx_in));
            index += 1;
        }
        Self {
            data_sender: data_tx,
            _inner_handles: handles,
        }
    }

    pub(crate) fn work_loop(rx: Receiver<Handler>) {
        loop {
            if let Ok(mut handler) = rx.recv() {
                handler.handle();
            }
        }
    }

}
