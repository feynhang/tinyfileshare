use std::{net::SocketAddr, path::PathBuf};

use clap::Parser;

// #[derive(Debug, Clone)]
// struct Host {
//     name: String,
//     address: SocketAddr,
// }

#[derive(Debug, clap::Subcommand)]
enum SubCommands {
    // #[command(override_usage("share.exe reg --name <NAME> --address <ADDRESS>\r\n       share.exe reg <NAME:HOST>"))]
    Reg {
        #[arg(short('n'), required(true))]
        hostname: String,
        #[arg(short('a'), required(true))]
        socketaddr: SocketAddr,
    },
}

#[derive(Debug, Parser)]
#[command(args_conflicts_with_subcommands(true), subcommand_negates_reqs(true))]
struct AppArgs {
    #[arg(short('n'), long, required(true))]
    hostname: Option<String>,
    #[arg(required(true), value_name("PATH"), num_args(1..=4))]
    files_paths: Vec<PathBuf>,
    #[command(subcommand)]
    reg_command: Option<SubCommands>,
}

fn main() {
    let args = AppArgs::parse();
    if let Some(hostname) = args.hostname {
        println!("Hostname is {}", hostname);

        for (i, p) in args.files_paths.iter().enumerate() {
            println!("The {}th path is {}", i, p.to_string_lossy());
        }
    }
    if let Some(SubCommands::Reg {
        hostname: name,
        socketaddr: address,
    }) = args.reg_command
    {
        println!("Accept reg_command, name = {}, addr = {}", name, address);
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
