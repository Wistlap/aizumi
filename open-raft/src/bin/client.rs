use std::{io::{Read, Write}, net::TcpStream};
use hoge::Msg;

fn main() {
    // TCP connection to the server.
    let mut stream = TcpStream::connect("127.0.0.1:8080").unwrap();
    let mut buf = [0; 1024];

    for i in 0..100_u16 {
        let msg = Msg {
            key: i,
            value: "Hello, World!".to_string(),
        };
        let msg_raw = bincode::serialize(&msg).unwrap();
        println!("Sending: {:?}", msg);
        stream.write_all(&msg_raw).unwrap();
        let n = stream.read(&mut buf).unwrap();
        // 受信したバイナリメッセージを文字列に変換
        let ack: Msg = bincode::deserialize(&buf[..n]).unwrap();
        println!("Received: {:?}", ack);
    }
}