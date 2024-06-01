use std::{
    net::{IpAddr, SocketAddr, TcpListener},
    path::{Path, MAIN_SEPARATOR},
};

use crossbeam::channel::bounded;

use crate::{
    config::Config, dispatcher::Dispatcher, error::CommonError, global, workers::Workers, CommonResult
};

static mut RUNNING: bool = false;


fn check_running() {
    unsafe {
        if RUNNING {
            panic!("Illegal function call!!!");
        }
        RUNNING = true;
    }
}

fn check_path(path_for_check: &Path) -> CommonResult<&Path> {
    const PATH_INVALID_CHAR: [char; 9] = ['\\', '*', '?', '/', '"', '<', '>', '|', ':'];
    if path_for_check.is_symlink() {
        return Err(CommonError::PathErr(
            "symbolic links for config path is not supported!".into(),
        ));
    }
    let path_str = path_for_check.to_str();
    if path_str.is_none() {
        return Err(CommonError::PathErr(format!(
            "The path is invalid unicode! path: {:?}",
            path_for_check
        )));
    }
    let invalid = path_str
        .unwrap()
        .split(MAIN_SEPARATOR)
        .any(|part| part.chars().any(|ch| PATH_INVALID_CHAR.contains(&ch)));
    if invalid {
        return Err(CommonError::PathErr(
            r#"Invalid path! Path should not contain these chars: \, *, ?, /, ", <, >, |, : "#
                .into(),
        ));
    }

    Ok(path_for_check)
}

fn start_inner() -> CommonResult<()> {
    let default_config = Config::default();
    let listener;
    let listen_res = TcpListener::bind(global::config().socket_addr());
    if let Err(e) = listen_res {
        if *global::config() == default_config {
            return Err(e.into());
        }
        *global::config() = default_config;
        listener = TcpListener::bind(global::config().socket_addr())?;
    } else {
        listener = listen_res.unwrap();
    }
    let local_addr = listener.local_addr().unwrap();
    global::config().set_addr(local_addr);
    global::config().store()?;
    let (handler_tx, handler_rx) = bounded(global::config().num_workers() as usize + 1);
    let mut dispatcher = Dispatcher::new(handler_tx);
    _ = Workers::start(handler_rx);
    loop {
        let accept_res  =listener.accept();
        if let Err(e) = accept_res  {
            global::logger().log(format_args!("Accept a connection failed!\nDetail: {}", e), crate::log::LogLevel::Error);
            continue;
        }
        let (conn, addr) = accept_res.unwrap();
        dispatcher.dispatch(conn, addr);
    }
    Ok(())
}

pub fn start_with_config_path<P: AsRef<Path>>(config_path: P) -> CommonResult<()> {
    check_running();
    crate::global::set_config_path(check_path(config_path.as_ref())?);
    *crate::global::config() = Config::from_file(config_path.as_ref());
    start_inner()?;
    Ok(())
}

pub fn start_default() -> CommonResult<()> {
    check_running();
    start_inner()?;
    Ok(())
}

pub fn start_on(addr: SocketAddr) -> CommonResult<()> {
    check_running();
    crate::global::config().set_addr(addr);
    start_inner()?;
    Ok(())
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
