extern crate websocket;
#[macro_use]
extern crate failure;

mod sync;

use std::net::SocketAddr;

fn main() {
    let ws_addr = "127.0.0.1:12345".parse().unwrap();
    let tcp_server_addr = "127.0.0.1:7900".parse().unwrap();

    sync::proxy(ws_addr, tcp_server_addr);
}