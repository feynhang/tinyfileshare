use fshare_server::server;

fn main() {
    let server = server::Server::default();
    if let Err(e) = server.start(){
        eprintln!("Start server failed: {}", e);
    }
}
