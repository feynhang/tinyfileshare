use std::{
    net::{SocketAddr, TcpListener, ToSocketAddrs},
    sync::OnceLock,
};

use crossbeam::channel::{bounded, Sender};

use crate::{
    config::Config, dispatcher::Dispatcher, handler::PathHandler, workers::Workers, CommonResult,
};

static mut SERVER: OnceLock<Server> = OnceLock::new();

pub struct Server {
    listener: TcpListener,
    current_config: Config,
    dispatcher: Dispatcher,
    _workers: Workers,
}

impl Server {
    pub fn start_with_config(mut config: Config) -> CommonResult<()> {
        let listener = TcpListener::bind((config.ip, config.port))?;
        let local_addr = listener.local_addr().unwrap();
        config.port = local_addr.port();
        config.ip = local_addr.ip();
        let num_workers = config.num_workers;
        let chan_size = config.num_workers as usize + 1;
        let (handler_tx, handler_rx) = bounded(chan_size);
        let (data_tx, data_rx) = bounded(chan_size);
        let server = unsafe {
            SERVER.get_or_init(|| Self {
                listener,
                current_config: config,
                dispatcher: Dispatcher::new(handler_tx, data_rx),
                _workers: Workers::start(handler_rx, data_tx, num_workers),
            })
        };
        while let Ok((stream, remote_addr)) = server.listener.accept() {
            server.dispatcher.dispatch(stream, remote_addr);
        }
        Ok(())
    }

    pub fn start_default() -> CommonResult<()> {
        Self::start_with_config(Config::default())?;
        Ok(())
    }

    pub fn start_on(addrs: SocketAddr) -> CommonResult<()> {
        let mut current_config = Config::from_socket_addr(addrs);
        if let Err(e) = Self::start_with_config(current_config) {
            
            Self::start_default()?
        }
        Ok(())
    }
    // pub fn listen_on_port(port: u16) -> Self {
    //     if let Ok(listener) = TcpListener::bind((Config::DEFAULT.ip, port)) {
    //         Self {
    //             listener,
    //             current_config: Config {
    //                 port,
    //                 ..Config::DEFAULT
    //             },
    //         }
    //     } else {
    //         Self::listen_default()
    //     }
    // }

    // pub fn listen_on<A: ToSocketAddrs>(addr: A) -> Self {
    //     if let Ok(listener) = TcpListener::bind(addr) {
    //         let local_addr = listener.local_addr().unwrap();
    //         let current_config = Config {
    //             port: local_addr.port(),
    //             ip: local_addr.ip(),
    //             ..Config::DEFAULT
    //         };
    //         Self {
    //             listener,
    //             current_config,
    //         }
    //     } else {
    //         Self::listen_default()
    //     }
    // }
}

#[cfg(test)]
mod tests {
    use std::{mem::transmute, path::PathBuf};

    use crate::handler::PathHandler;

    #[test]
    fn ptr_test() {
        let v: u16 = 8905;
        let p_v: *const u16 = &v;
        unsafe {
            let p_u8 = transmute::<*const u16, *const u8>(p_v);

            let part1 = *p_u8;
            let part2 = *p_u8.add(1);

            println!("rep_v1 = {}", part1);
            println!("rep_v2 = {}", part2);

            let bytes = [part1, part2];
            let p_bytes = bytes.as_ptr();
            let p_raw_v = transmute::<*const u8, *const u16>(p_bytes);

            assert_eq!(v, *p_raw_v);
        }
    }

    #[test]
    fn to_ne_bytes_test() {
        let v = 8905_u16;
        let bytes = v.to_ne_bytes();
        println!("{}\n{}", bytes[0], bytes[1]);
        assert_eq!(v, u16::from_ne_bytes(bytes));
    }

    #[test]
    fn create_dir_all_test() {
        let mut home_path = PathBuf::from(PathHandler::get_home_path());
        home_path.push(".test");
        home_path.push("innerdir1");
        home_path.push("inner_dir2");
        let res = std::fs::create_dir_all(home_path);
        assert!(res.is_ok());
    }
}
