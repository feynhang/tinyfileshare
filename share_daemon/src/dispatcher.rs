use std::{
    io::{BufRead, BufReader, BufWriter},
    net::{SocketAddr, TcpStream},
};

use crossbeam::channel::Sender;

use crate::{
    error::CommonError,
    global,
    handler::{Handler, WriteLine},
    request::{Request, RequestCommand},
    response::Response,
    CommonResult,
};

pub struct Dispatcher {
    handler_tx: Sender<Handler>,
}

impl Dispatcher {
    pub(crate) fn new(handler_tx: Sender<Handler>) -> Self {
        Self { handler_tx }
    }

    pub(crate) fn dispatch(
        &mut self,
        connection: TcpStream,
        peer_addr: SocketAddr,
    ) -> CommonResult<()> {
        let conn = connection.try_clone()?;
        let req_res = Request::try_from(connection.try_clone()?);
        let mut writer = BufWriter::with_capacity(global::BUF_SIZE, conn);
        if let Err(CommonError::ReplyErr(err_resp)) = req_res {
            writer.write_line(err_resp)?;
            return Ok(());
        }

        let req = req_res.unwrap();
        match req.kind() {
            RequestCommand::FileShare => todo!(),
            RequestCommand::HostRegistration => todo!(),
            RequestCommand::FileReceive => todo!(),
        }
        // let mut line = String::new();
        // let mut reader = BufReader::new(connection);
        // let read_size = reader.read_line(&mut line)?;
        // if read_size == 0 || line.trim().is_empty() {
        //     self.handler_tx
        //         .send(Handler::ReplyHandler(
        //             reader.into_inner(),
        //             Response::InvalidRequest,
        //         ))
        //         .unwrap();
        //     return Ok(());
        // }

        // match RequestCommand::from(line) {
        //     RequestCommand::FileShare => {
        //         if peer_addr.ip().is_loopback() {
        //             self.handler_tx
        //                 .send(Handler::FileShareHandler(reader))
        //                 .unwrap();
        //             return Ok(());
        //         }
        //     }
        //     RequestCommand::FileReceive => {
        //         if !peer_addr.ip().is_loopback() {
        //             self.handler_tx
        //                 .send(Handler::FileRecvHandler(reader))
        //                 .unwrap();
        //             return Ok(());
        //         }
        //     }
        //     RequestCommand::HostRegistration => {
        //         if peer_addr.ip().is_loopback() {
        //             self.handler_tx
        //                 .send(Handler::HostRegHandler(reader))
        //                 .unwrap();
        //             return Ok(());
        //         }
        //     }
        // }
        // self.handler_tx
        //     .send(Handler::ReplyHandler(
        //         reader.into_inner(),
        //         Response::InvalidRequest,
        //     ))
        //     .unwrap();
        Ok(())
    }
}
