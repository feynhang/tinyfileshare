use std::{
    ffi::OsStr,
    net::{IpAddr, Ipv4Addr},
    path::PathBuf,
};

use interprocess::local_socket::{
    traits::tokio::Listener, GenericNamespaced, ListenerOptions, NamespacedNameType,
};
use tokio::{net::TcpListener, task::JoinSet};

use crate::{
    config::{Config, ConfigStore},
    global, handler, CommonResult,
};

const NAME_IPC: &str = "tinyfileshare.localsock";

fn join_set() -> &'static mut JoinSet<()> {
    static mut JOIN_SET: Option<JoinSet<()>> = None;
    unsafe { JOIN_SET.get_or_insert(JoinSet::new()) }
}

static mut CONFIG: Option<ConfigStore> = None;

pub(crate) fn config_store() -> &'static mut ConfigStore {
    unsafe {
        match CONFIG.as_mut() {
            Some(conf_store) => {
                conf_store.update_config();
                conf_store
            }
            None => {
                CONFIG = Some(ConfigStore::from_config_path());
                CONFIG.as_mut().unwrap()
            }
        }
    }
}

async fn try_join() {
    while join_set().len() > config_store().number_of_workers() as usize {
        if let Some(Err(e)) = join_set().join_next().await {
            global::logger().error(format_args!(
                "A local request handler task join failed: {}",
                e
            ));
        }
    }
}

async fn start_local_daemon() {
    let name_res = GenericNamespaced::map(OsStr::new(NAME_IPC).into());
    if let Ok(listen_name) = name_res {
        let listener_res = ListenerOptions::new().name(listen_name).create_tokio();
        if let Ok(local_listener) = listener_res {
            loop {
                let conn = match local_listener.accept().await {
                    Ok(c) => c,
                    Err(e) => {
                        global::logger().warn(format_args!(
                            "There was an error with an incoming connection: {}",
                            e
                        ));
                        continue;
                    }
                };
                try_join().await;
                join_set().spawn(async move {
                    if let Err(e) = handler::handle_local(conn).await {
                        global::logger().error(format_args!(
                            "Error occurred while handling a local process connection: {}",
                            e
                        ));
                    }
                });
            }
        } else {
            let err = listener_res.unwrap_err();
            if err.kind() == tokio::io::ErrorKind::AddrInUse {
                global::logger().error(format_args!("Error: could not start server because the socket file is occupied. Please check if {} is in use by another process and try again.", NAME_IPC));
            } else {
                global::logger().error(format_args!(
                    "Error occurred while create ipc listener: {}",
                    err
                ));
            }
            std::process::exit(1);
        }
    } else {
        global::logger().error(format_args!(
            "Error occurred while create ipc socket name: {}",
            name_res.unwrap_err()
        ));
        std::process::exit(1);
    }
}

async fn start_inner(ip_addr: IpAddr) -> CommonResult<()> {
    let config = config_store();
    let default_config = Config::default();
    let listener;
    let listen_res =
        TcpListener::bind((ip_addr, config.start_port())).await;
    if let Err(e) = listen_res {
        if *config.inner_config() == default_config {
            return Err(e.into());
        }
        config.use_default();
        listener = TcpListener::bind((ip_addr, config.start_port())).await?;
    } else {
        listener = listen_res.unwrap();
    }
    let local_addr = listener.local_addr().unwrap();
    config.set_start_port(local_addr.port());
    ctrlc::set_handler(|| {
        println!("CtrlC Pressed, Exiting forced now!");
        std::process::exit(0);
    })
    .expect("Set Ctrl+C event handler failed!");
    tokio::spawn(start_local_daemon());
    loop {
        match listener.accept().await {
            Ok((socket, addr)) => {
                try_join().await;
                join_set().spawn(async move {
                    if let Err(e) = handler::handle_remote(socket, addr).await {
                        global::logger().error(format_args!(
                            "Error occurred while handling a remote connection: {}",
                            e
                        ));
                    }
                });
            }
            Err(e) => {
                global::logger().log(
                    format_args!("Accept connection error: {}", e),
                    crate::log::LogLevel::Error,
                );
            }
        }
    }
}

pub async fn start_with_config_path<P: Into<PathBuf>>(config_path: P) -> CommonResult<()> {
    global::set_config_path(config_path.into())?;
    *config_store() = ConfigStore::from_config_path();
    start_inner(IpAddr::V4(Ipv4Addr::UNSPECIFIED)).await?;
    Ok(())
}

pub async fn start_default() -> CommonResult<()> {
    start_inner(IpAddr::V4(Ipv4Addr::UNSPECIFIED)).await?;
    Ok(())
}

pub async fn start_on(addr: std::net::SocketAddr) -> CommonResult<()> {
    config_store().set_start_port(addr.port());
    start_inner(addr.ip()).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{mem::transmute, path::PathBuf};

    use crate::global;

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
        let mut home_path = PathBuf::from(global::home_path());
        home_path.push(".test");
        home_path.push("innerdir1");
        home_path.push("inner_dir2");
        let res = std::fs::create_dir_all(home_path);
        assert!(res.is_ok());
    }
}
