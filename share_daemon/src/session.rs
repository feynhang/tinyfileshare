use std::{
    io::{BufReader, BufWriter},
    net::TcpStream,
};

use uuid::Uuid;

pub struct Session {
    id: Uuid,
    // handler: 
}

impl Session {
    pub(crate) fn new(stream: TcpStream) -> Self {
        Self {
            // stream,
            id: Uuid::new_v4(),
        }
    }

    fn id(&self) -> Uuid {
        self.id
    }

    fn handle(&mut self) -> std::io::Result<()> {
        todo!()
    }
}
