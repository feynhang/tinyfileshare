use fshare_server::server;

fn main() {
    if let Err(e) = server::Server::default().start(){
        eprintln!("Start server failed: {}", e);
    }
}
