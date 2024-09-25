use std::{io::{Read, Write}, net::TcpStream};
use serde_json::json;
use std::time::Duration;


fn main() -> std::io::Result<()> {
    // サーバに接続
    let mut stream = TcpStream::connect("127.0.0.1:21010")?;
    println!("Connected to the server!");
    // stream.set_nodelay(true)?;
    // stream.set_nonblocking(true)?;

    for i in 0..3 {
        // 送信するJSONデータ
        // 奈良にを変えない
        let json_data = json!({
            "msg_type": "MSG_SEND_REQ",
            "saddr": 1,
            "daddr": 100,
            "id": i,
            "payload": "hello"
        }).to_string();
        // r#"{"msg_type": "MSG_SEND_REQ","saddr": 1,"daddr": 100,"id": 1,"payload": "hello"}"#;
        // JSONデータを送信
        let res = stream.write(json_data.as_bytes());
        if let Err(e) = res {
            eprintln!("Failed to send JSON: {}", e);
            return Err(e);
        }
        println!("Sent JSON: {}", json_data);

        stream.flush()?;

        // サーバからのレスポンスを受信
        let mut buffer = [0; 1024];
        let n = stream.read(&mut buffer)?;
        println!("Received from server: {}", String::from_utf8_lossy(&buffer[..n]));
        println!();
        // 少し待つ
        std::thread::sleep(Duration::from_secs(1));
    }

    println!("Finished sending JSON data.");

    Ok(())
}
