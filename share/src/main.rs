use std::{net::SocketAddr, path::PathBuf};

use clap::Parser;

#[derive(Debug, clap::Subcommand)]
enum SubCommands {
    Reg {
        #[arg(short('n'), required(true), help("An unique hostname used as ID (at least on this machine).\nAnd its length must be less than 16 byte long"))]
        hostname: String,
        #[arg(short('a'), required(true), help("The network address of the host"))]
        hostaddr: SocketAddr,
    },
}

#[derive(Debug, Parser)]
#[command(args_conflicts_with_subcommands(true), subcommand_negates_reqs(true))]
struct AppArgs {
    #[arg(short('n'), long, required(true), help("A hostname refers to a registered host (a certain network address).\nThe host must been registered on the remote side.\nIMPORTANT: its length limit is 16\n"))]
    hostname: Option<String>,
    #[arg(required(true), value_name("PATH"), num_args(1..=4))]
    files_paths: Vec<PathBuf>,
    #[command(subcommand)]
    reg_command: Option<SubCommands>,
}

fn main() {
    let args = AppArgs::parse();
    if let Some(SubCommands::Reg { hostname, hostaddr }) = args.reg_command {
        println!(
            "Accept reg_command, hostname = {}, addr = {}",
            hostname, hostaddr
        );
    } else {
        println!("Accept root command, hostname = {}", args.hostname.unwrap());

        for (i, p) in args.files_paths.iter().enumerate() {
            println!("{}th path = {}", i, p.to_string_lossy());
        }
    }
}

#[cfg(test)]
mod tests {
    use std::net::SocketAddr;

    #[test]
    fn print_cargo_pkg_name() {
        println!(std::env!("CARGO_PKG_NAME"))
    }

    #[test]
    fn socket_parse_test() {
        let addr_str = "192.168.3.40:10020";
        let res = addr_str.parse::<SocketAddr>();
        assert!(res.is_ok())
    }
}
