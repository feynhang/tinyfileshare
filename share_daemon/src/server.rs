use std::{ffi::OsStr, path::PathBuf};

use interprocess::local_socket::{
    traits::tokio::Listener, GenericNamespaced, ListenerOptions, NamespacedNameType,
};
use smol_str::{format_smolstr, SmolStr};
use tokio::{net::TcpListener, task::JoinSet};

use crate::{
    config::Config,
    consts, global, handler,
    log::{LogLevel, Logger, LoggerKind},
    CommonResult,
};

fn join_set() -> &'static mut JoinSet<()> {
    static mut JOIN_SET: Option<JoinSet<()>> = None;
    unsafe { JOIN_SET.get_or_insert(JoinSet::new()) }
}

pub struct Server {
    local_pipe_name: SmolStr,
    logger_kind: LoggerKind,
    log_dir: PathBuf,
    log_level: LogLevel,
    config: Config,
}

impl Server {
    pub fn default() -> Self {
        Self {
            local_pipe_name: SmolStr::new_inline(consts::NAME_IPC_LISTENER),
            logger_kind: LoggerKind::ConsoleLogger,
            log_level: LogLevel::Warn,
            log_dir: global::default_log_dir().to_owned(),
            config: Config::default(),
        }
    }
    pub fn local_pipe_name(&mut self, name: &str) -> &mut Self {
        self.local_pipe_name = SmolStr::new_inline(name);
        self
    }

    pub fn listener_port(&mut self, port: u16) -> &mut Self {
        self.config.set_listener_port(port);
        self
    }

    pub fn log_level(&mut self, level: LogLevel) -> &mut Self {
        self.log_level = level;
        self
    }

    pub fn log_dir<P: Into<PathBuf>>(&mut self, log_dir: P) -> &mut Self {
        self.log_dir = log_dir.into();
        self
    }

    pub fn use_config_file<P: Into<std::path::PathBuf>>(
        &mut self,
        config_file_path: P,
    ) -> CommonResult<&mut Self> {
        global::set_config_path(config_file_path.into())?;
        Ok(self)
    }

    pub fn num_workers(&mut self, n: u8) -> &mut Self {
        self.config.set_num_workers(n);
        self
    }

    pub fn use_console_logger(&mut self) -> &mut Self {
        self.logger_kind = LoggerKind::ConsoleLogger;
        self
    }

    pub fn use_file_logger(&mut self) -> &mut Self {
        self.logger_kind = LoggerKind::FileLogger;
        self
    }

    pub fn preset_receive_dir<P: Into<PathBuf>>(&mut self, receive_dir: P) -> &mut Self {
        self.config.set_file_save_dir(receive_dir);
        self
    }

    pub fn start(self) -> CommonResult<()> {
        tokio::runtime::Builder::new_multi_thread()
            .enable_io()
            .build()
            .unwrap()
            .block_on(self.start_inner())
    }

    async fn try_join() {
        while join_set().len()
            > global::config_store()
                .await
                .read()
                .await
                .inner()
                .num_workers as usize
        {
            if let Some(Err(e)) = join_set().join_next().await {
                global::logger()
                    .error(format_smolstr!(
                        "A local request handler task join failed: {}",
                        e
                    ))
                    .await;
            }
        }
    }

    async fn start_local_daemon(pipe_name: SmolStr) {
        let name_res = GenericNamespaced::map(OsStr::new(pipe_name.as_str()).into());
        if let Ok(listen_name) = name_res {
            let mut listener_res = ListenerOptions::new().name(listen_name).create_tokio();
            if pipe_name != consts::NAME_IPC_LISTENER && listener_res.is_err() {
                global::logger().warn(format_smolstr!("Create local listener failed, using specified pipe_name: {pipe_name}. Try to create it using default pipe name...")).await;
                listener_res = ListenerOptions::new()
                    .name(
                        GenericNamespaced::map(OsStr::new(consts::NAME_IPC_LISTENER).into())
                            .expect("Create GenericNamespace for local listener failed!"),
                    )
                    .create_tokio();
            }
            if let Ok(local_listener) = listener_res {
                loop {
                    let conn = match local_listener.accept().await {
                        Ok(c) => c,
                        Err(e) => {
                            global::logger()
                                .warn(format_smolstr!(
                                    "There was an error with an incoming connection: {}",
                                    e
                                ))
                                .await;
                            continue;
                        }
                    };
                    Self::try_join().await;
                    join_set().spawn(async move {
                        if let Err(e) = handler::handle_local(conn).await {
                            global::logger()
                                .error(format_smolstr!(
                                    "Error occurred while handling a local process connection: {}",
                                    e
                                ))
                                .await;
                        }
                    });
                }
            } else {
                let err = listener_res.unwrap_err();
                if err.kind() == tokio::io::ErrorKind::AddrInUse {
                    global::logger().error(format_smolstr!("Error: could not start server because the socket file is occupied. Please check if {} is in use by another process and try again.", consts::NAME_IPC_LISTENER)).await;
                } else {
                    global::logger()
                        .error(format_smolstr!(
                            "Error occurred while create ipc listener: {}",
                            err
                        ))
                        .await;
                }
                std::process::exit(1);
            }
        } else {
            global::logger()
                .error(format_smolstr!(
                    "Error occurred while create ipc socket name: {}",
                    name_res.unwrap_err()
                ))
                .await;
            std::process::exit(1);
        }
    }

    async fn start_inner(mut self) -> CommonResult<()> {
        unsafe {
            global::GLOBAL_LOGGER = match self.logger_kind {
                LoggerKind::FileLogger => Logger::file_logger(),
                LoggerKind::ConsoleLogger => Logger::console_logger(),
                LoggerKind::NoLogger => Logger::no_logger(),
            };
        }
        *global::log_dir() = self.log_dir;
        let default_config = Config::default();
        let listener;
        let listen_res = TcpListener::bind(self.config.listener_addr).await;
        if let Err(e) = listen_res {
            if self.config.listener_addr == default_config.listener_addr {
                return Err(e.into());
            }
            listener = TcpListener::bind(default_config.listener_addr).await?;
        } else {
            listener = listen_res.unwrap();
        }
        let local_addr = listener.local_addr().unwrap();
        self.config.set_listener_addr(local_addr);
        let conf_store_lock = global::config_store().await;
        let mut config_store = conf_store_lock.write().await;
        config_store.set_config(self.config);
        config_store.save_to_file()?;
        ctrlc::set_handler(|| {
            println!("CtrlC Pressed, Exiting forced now!");
            std::process::exit(0);
        })
        .expect("Set Ctrl+C event handler failed!");
        tokio::spawn(Self::start_local_daemon(self.local_pipe_name));
        loop {
            match listener.accept().await {
                Ok((socket, addr)) => {
                    Self::try_join().await;
                    join_set().spawn(async move {
                        if let Err(e) = handler::handle_remote(socket, addr).await {
                            global::logger()
                                .error(format_smolstr!(
                                    "Error occurred while handling a remote connection: {}",
                                    e
                                ))
                                .await;
                        }
                    });
                }
                Err(e) => {
                    global::logger()
                        .log(
                            format_smolstr!("Accept connection error: {}", e),
                            crate::log::LogLevel::Error,
                        )
                        .await;
                }
            }
        }
    }
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
