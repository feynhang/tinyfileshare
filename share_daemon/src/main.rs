use fshare_server::server;

fn main() {
    let tokio_rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    if let Err(e) = tokio_rt.block_on(server::start_default()) {
        eprintln!("Start server failed: {}", e);
    }
}
