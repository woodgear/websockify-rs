use std::thread;
use websocket::OwnedMessage;
use websocket::sync::Server;
use websocket::sync::server::AcceptResult;
use websocket::sync::Client as WsSyncClient;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpStream};
use std::io;
use std::sync::mpsc::{Sender, Receiver, channel};
use std::io::Read;
use std::io::Write;
use std::net::Shutdown;


const BUFF_SIZE: usize = 2048;

type WsConnection = WsSyncClient<TcpStream>;

#[derive(Debug, Fail)]
enum TcpScoketErr {
    #[fail(display = "read 0 lenght of bytes")]
    ReadZero,
    #[fail(display = "{}", _0)]
    SocketErr(#[cause] io::Error),
}

enum SocketEvent {
    Data(Vec<u8>),
    Error(TcpScoketErr),
}

fn handle_ws_request(connection: WsConnection, tcp_server_addr: SocketAddr) {
//    thread::spawn(move || {
    let mut ws_client = connection;

    let ip = ws_client.peer_addr().unwrap();

    println!("Connection from {}", ip);

    let (mut ws_client_receiver, mut ws_client_sender) = ws_client.split().unwrap();
    let mut tcp_client = TcpStream::connect(tcp_server_addr)
        .expect("couldn't bind to address");
    let mut tcp_client_clone = tcp_client.try_clone().unwrap();

    thread::spawn(move || {
        for message in ws_client_receiver.incoming_messages() {
            let message = message;
            println!("recv message");
            match message {
                Ok(OwnedMessage::Close(_)) => {
                    println!("WS Client {} disconnected", ip);
                    tcp_client_clone.shutdown(Shutdown::Both);
                    return;
                }
                Ok(OwnedMessage::Binary(data)) => {
                    println!("ws recv data", );
                    tcp_client_clone.write(&data);
                }
                Ok(OwnedMessage::Text(data)) => {
                    println!("ws recv text {}", data);
                }
                Ok(other) => {
                    println!("recv other {:?}", other);
                }

                Err(e) => {
                    println!("ws client msg error {:?}", e);
                }
            }
        }
    });

    let mut tcp_client_clone = tcp_client.try_clone().unwrap();
    thread::spawn(move || {
        let mut buf = [0; BUFF_SIZE];
        loop {
            println!("on loop");
            match tcp_client_clone.read(&mut buf) {
                Ok(0) => {
                    println!("tcp recv 0 lenth bytes");
//                    tcp_client_clone.shutdown(Shutdown::Both);
//                    return;
                }
                Ok(size) => {
                    println!("tcp recv {} lenth bytes ", size);
                    ws_client_sender.send_message(&OwnedMessage::Binary(buf[0..size].to_vec()));
                }
                Err(e) => {
                    println!("tcp recv error {}", e);
//                    tcp_client_clone.shutdown(Shutdown::Both);
//                    return;
                }
            }
        };
    });
}

pub fn proxy(ws_addr: SocketAddr, tcp_server_addr: SocketAddr) {
    println!("proxy tcp server {:?} to websocket server {:?}", tcp_server_addr, ws_addr);
    let server = Server::bind(ws_addr).unwrap();

    for (request_index, request) in server
        .filter_map(Result::ok)
        .map(|request| request.use_protocol("binary").accept())
        .filter_map(Result::ok)
        .enumerate() {
        thread::spawn(move || {
            handle_ws_request(request, tcp_server_addr);
        });
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;
    use std::io::BufReader;
    use websocket::ClientBuilder;
    use std::io::BufRead;
    use failure::Error;

    fn tcp_echo_server(addr: SocketAddr) {
        let listener = TcpListener::bind(addr).unwrap();
        println!("server is running on {:?}", addr);

        for stream in listener.incoming() {
            let mut stream = stream.unwrap();
            let mut buf = [0; 1024];
            stream.read(&mut buf).unwrap();
            stream
                .write(&buf)
                .unwrap();
        };
    }

    fn tcp_get(addr: SocketAddr, msg: &Vec<u8>) -> Result<Vec<u8>, Error> {
        let mut stream = TcpStream::connect(addr)?;
        let _ = stream.write(&msg);
        let mut buf = vec![];
        let _ = stream.read_to_end(&mut buf)?;
        Ok(buf)
    }


    fn ws_get(url: &str, msg: &Vec<u8>) -> Result<Vec<u8>, String> {
        let client = ClientBuilder::new(url)
            .unwrap()
            .connect_insecure()
            .unwrap();

        let (mut receiver, mut sender) = client.split().unwrap();
        sender.send_message(&OwnedMessage::Binary(msg.to_vec()));
        sender.send_message(&OwnedMessage::Close(None));

        let res_msg = receiver.incoming_messages().nth(0).unwrap().unwrap();

        match res_msg {
            OwnedMessage::Binary(data) => {
                Ok(data)
            }
            _ => {
                Err(format!("not this data {:?}", res_msg))
            }
        }
    }

    #[test]
    fn test_proxy() {
        let ws_addr = "127.0.0.1:12345".parse().unwrap();
        let tcp_server_addr = "127.0.0.1:7900".parse().unwrap();
        let ws_url = "ws://127.0.0.1:12345";

        thread::spawn(move || {
            proxy(ws_addr, tcp_server_addr);
        });
        thread::spawn(move || {
            tcp_echo_server(tcp_server_addr);
        });
        thread::sleep_ms(1000 * 3);
        let msg = "test";
//        let msg = r#"GET / HTTP/1.1
//Host: 127.0.0.1:12345
//Connection: Upgrade
//Upgrade: websocket
//Sec-WebSocket-Version: 13
//Sec-WebSocket-Key: Zz1CsLUhH9l0VsilFB5eZQ=="#;
        println!("msg is {}", msg);
        let msg = msg.as_bytes().to_vec();
        println!("msg byte len is {}", msg.len());

//        loop {
        let res = ws_get(ws_url, &msg);
//        let res = tcp_get(tcp_server_addr, &msg);
        println!("res is -> {}", String::from_utf8(res.unwrap()).unwrap());
        thread::sleep_ms(1000 * 3);
//        }
    }
}