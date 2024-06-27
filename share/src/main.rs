#![allow(unused)]
use std::{net::SocketAddr, ops::Deref, path::PathBuf};

use clap::{value_parser, Arg, ArgAction, Command};
use smol_str::SmolStr;

#[derive(Debug, Clone)]
struct Hostname(SmolStr);

impl Deref for Hostname {
    type Target = SmolStr;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::fmt::Display for Hostname {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl std::str::FromStr for Hostname {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() || s.len() > fshare_server::consts::FILE_NAME_LENGTH_LIMIT {
            return Err(anyhow::anyhow!(
                "The length of the host name must be in range  1..={}!",
                fshare_server::consts::FILE_NAME_LENGTH_LIMIT
            ));
        }
        Ok(Self(s.into()))
    }
}

mod id {
    pub const HOSTNAME: &str = "hostname";
    pub const PATH: &str = "PATH";
    pub const ADDRESS: &str = "address";
    pub const LOCAL_ONLY: &str = "local_only";
}

fn main() {
    let matches = Command::new(env!("CARGO_CRATE_NAME"))
        .arg(Arg::new(id::HOSTNAME).short('n').long(id::HOSTNAME).required(true).value_parser(value_parser!(Hostname)).help(color_print::cstr!("A hostname refers to a registered host (a network address), \nthe host should been registered on the remote side. \nThose hosts already registered could be found in a config file.\n Default config path is <bold>$HOME/.tinyfileshare/.config.toml</bold>.\n<bold>NOTE: It can not be empty, its length can not be out of 20 bytes</bold>")))
        .arg(
            Arg::new("PATH")
                .num_args(1..=4)
                .required(true)
                .value_parser(value_parser!(PathBuf))
                .action(ArgAction::Append).help("The paths of the files shared to the remote host (with the given hostname). \nThe number range of paths is 1..=4."),
        )
        .subcommand(
            Command::new("reg").short_flag('r')
                .about("Register a host with hostname")
                .arg(Arg::new(id::HOSTNAME).short('n').long(id::HOSTNAME).required(true).value_parser(value_parser!(Hostname)).help(color_print::cstr!("An unique hostname used as ID(at least on this machine) for the host. \n<bold>NOTE: It can not be empty, its length can not be out of 20 bytes</bold>")))
                .arg(Arg::new(id::ADDRESS).short('a').long(id::ADDRESS).required(true).value_parser(value_parser!(SocketAddr)).help("The network address within port of the host. Such as 192.168.1.2:20"))
                .arg(Arg::new(id::LOCAL_ONLY).short('l').long("local").action(ArgAction::SetTrue).value_parser(value_parser!(bool)).help("Register the given to local only. \nWhich actually means writing hostname and address to local configuration file only.")),
        ).args_conflicts_with_subcommands(true).get_matches();
    match matches.subcommand() {
        Some((_, sub_matches)) => {
            let hostname = sub_matches.get_one::<Hostname>(id::HOSTNAME).unwrap();
            let address = sub_matches.get_one::<SocketAddr>(id::ADDRESS).unwrap();
            let local_only = sub_matches.get_flag(id::LOCAL_ONLY);
            println!("hostname = {}, address = {}, local_only = {}", hostname, address, local_only);
        }
        None => {
            let hostname = matches.get_one::<Hostname>(id::HOSTNAME).unwrap();
            println!("No subcommand, hostname = {}", hostname);
            for (idx, p) in matches.get_many::<PathBuf>(id::PATH).unwrap().enumerate() {
                println!("The {}th path: {}", idx, p.to_string_lossy());
            }
        }
    }

}

// fn connect_daemon() -> anyhow::Result<>

fn share_files() -> anyhow::Result<()> {
    todo!()
}

fn reg_host(local: bool) -> anyhow::Result<()> {
    todo!()
}

#[cfg(test)]
mod tests {
    use std::net::SocketAddr;

    #[test]
    fn print_cargo_pkg_name() {
        println!("CARGO_PKG_NAME = {}", env!("CARGO_PKG_NAME"));
        println!("CARGO_CRATE_NMAE = {}", env!("CARGO_CRATE_NAME"));
    }

    #[test]
    fn socket_parse_test() {
        let addr_str = "192.168.3.40:10020";
        let res = addr_str.parse::<SocketAddr>();
        assert!(res.is_ok())
    }
}
