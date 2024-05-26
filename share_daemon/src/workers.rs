use std::
    thread::{self, JoinHandle}
;

use crossbeam::channel::Receiver;

use crate::handler::Handler;

#[derive(Debug)]
pub struct Workers {
    _inner_handles: Vec<JoinHandle<()>>,
}

impl Workers {
    pub(crate) fn start(
        handler_rx: Receiver<Handler>,
        // result_tx: Sender<HandleResult>,
        num_workers: u8,
    ) -> Self {
        let mut index = 0;
        let mut handles = vec![];
        while index <= num_workers as usize {
            let rx_in = handler_rx.clone();
            // let tx_in = result_tx.clone();
            handles[index] = thread::spawn(move || Self::work_loop(rx_in));
            index += 1;
        }
        Self {
            _inner_handles: handles,
        }
    }

    pub(crate) fn work_loop(handler_rx: Receiver<Handler>) {
        loop {
            if let Ok(handler) = handler_rx.recv() {
                handler.handle().unwrap();
                // result_tx.send(result).unwrap();
            }
        }
    }
}
