use std::{io::{Read, Write}, net::TcpStream};
use serde_json::json;
use std::time;

pub use clap::Parser;
use std::fmt::Display;

/// Command-line Argument of m-broker-rs
#[derive(Parser, Debug)]
#[command(author, version, about)]
pub struct Args {

    #[arg(short = 'm', long, default_value_t = String::from("MSG_PUSH_ACK"))]
    pub msg_type: String,

    #[arg(short = 's', long, default_value_t = 100)]
    pub saddr: u32,

    #[arg(short = 'd', long, default_value_t = 5000)]
    pub daddr: u32,

    #[arg(short = 'i', long, default_value_t = 0)]
    pub id: u32,

    #[arg(short = 'b', long, default_value_t = String::from("127.0.0.1:21101"))]
    pub baddr: String,

    #[arg(short = 'l', long, default_value_t = 1)]
    pub loop_times: u32,
}

impl Display for Args {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let output = format!("msg_type: {}\n", self.msg_type);
        let output = format!("{output}saddr: {}\n", self.saddr);
        let output = format!("{output}daddr: {}\n", self.daddr);
        let output = format!("{output}id: {}\n", self.id);
        let output = format!("{output}baddr: {}\n", self.baddr);
        let output = format!("{output}loop_times: {}\n", self.loop_times);
        write!(f, "{}", output)
    }
}


fn main() -> std::io::Result<()> {
    let args = Args::parse();
    // サーバに接続
    let mut stream = TcpStream::connect(args.baddr)?;
    // stream.set_nodelay(true)?;
    // stream.set_nonblocking(true)?;

    let json_data = json!({
        "msg_type": "MSG_HELO_REQ",
        "saddr": args.saddr,
        "daddr": args.daddr,
        "id": 0,
        "payload": "hello"
    }).to_string();

    // JSONデータを送信
    let res = stream.write(json_data.as_bytes());
    if let Err(e) = res {
        eprintln!("Failed to send JSON: {}", e);
        return Err(e);
    }

    stream.flush()?;

    // サーバからのレスポンスを受信
    let mut buffer = [0; 1024];
    let _n = stream.read(&mut buffer)?;
    // 受信したメッセージを表示
    let msg = String::from_utf8_lossy(&buffer).to_string();
    println!("{}", msg);

    let n = args.id + args.loop_times;

    let start = time::Instant::now();
    for _i in args.id..n {

        println!("{}",_i);
        // サーバからのリクエストを受信
        let _n = stream.read(&mut buffer)?;
        let msg = String::from_utf8_lossy(&buffer).to_string();
        println!("{}", msg);
        // let req: serde_json::Value = serde_json::from_str(&msg).unwrap();

        // 送信するJSONデータ(レスポンス)
        let response = json!({
            "msg_type": args.msg_type,
            "saddr": args.saddr,
            "daddr": args.daddr,
            "id": 0,
            "payload": ""
        }).to_string();

        // JSONデータを送信
        let res = stream.write(response.as_bytes());
        if let Err(e) = res {
            eprintln!("Failed to send JSON: {}", e);
            return Err(e);
        }
        stream.flush()?;
        // 送信したメッセージを表示
        println!("{}", response);
    }
    let elapsed = start.elapsed();
    println!("(receiver) Elapsed: {}.{:03} seconds", elapsed.as_secs(), elapsed.subsec_millis());

    Ok(())
}
