use std::{
    net::{SocketAddr, TcpListener},
    path::{Path, PathBuf, MAIN_SEPARATOR},
};

use crossbeam::channel::bounded;

use crate::{
    config::Config, dispatcher::Dispatcher, error::ServerError, workers::Workers, ServerResult,
};

pub struct Server;

impl Server {
    fn checked_path(path: &Path) -> ServerResult<&Path> {
        const PATH_INVALID_CHAR: [char; 9] = ['\\', '*', '?', '/', '"', '<', '>', '|', ':'];
        if path.is_symlink() {
            return Err(ServerError::PathError(
                "symbolic links for config path is not supported!",
            ));
        }
        let invalid = path
            .to_str()
            .unwrap()
            .split(MAIN_SEPARATOR)
            .any(|part| part.chars().any(|ch| PATH_INVALID_CHAR.contains(&ch)));
        if invalid {
            return Err(ServerError::PathError("Invalid path!"));
        }

        Ok(path)
    }

    fn start_with_config(mut config: Config) -> ServerResult<()> {
        let default_config = Config::default();
        let listener;
        let listen_res = TcpListener::bind((config.ip, config.port));
        if let Err(e) = listen_res {
            if config == default_config {
                return Err(e.into());
            }
            config = default_config;
            listener = TcpListener::bind((config.ip, config.port))?;
        } else {
            listener = listen_res.unwrap();
        }
        let local_addr = listener.local_addr().unwrap();
        config.port = local_addr.port();
        config.ip = local_addr.ip();
        let num_workers = config.num_workers;
        config.store_to_file();
        let (handler_tx, handler_rx) = bounded(num_workers as usize + 1);
        let mut dispatcher = Dispatcher::new(handler_tx);
        _ = Workers::start(handler_rx, num_workers);
        while let Ok((stream, _)) = listener.accept() {
            dispatcher.dispatch(stream)?;
        }
        Ok(())
    }

    pub fn start_with_config_path(config_path: &Path) -> ServerResult<()> {
        crate::global::set_config_path(Self::checked_path(config_path)?);
        Self::start_with_config(Config::try_from(config_path)?)?;
        Ok(())
    }

    pub fn start_default() -> ServerResult<()> {
        Self::start_with_config(Config::default())?;
        Ok(())
    }

    pub fn start_on(addr: SocketAddr) -> ServerResult<()> {
        let current_config = Config {
            ip: addr.ip(),
            port: addr.port(),
            ..Default::default()
        };
        Self::start_with_config(current_config)?;
        Ok(())
    }
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
