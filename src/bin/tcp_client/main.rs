use std::{io::{Read, Write}, net::TcpStream};
use serde_json::json;
use std::time;

pub use clap::Parser;
use std::fmt::Display;

/// Command-line Argument of m-broker-rs
#[derive(Parser, Debug)]
#[command(author, version, about)]
pub struct Args {

    #[arg(short = 'm', long, default_value_t = String::from("MSG_SEND_REQ"))]
    pub msg_type: String,

    #[arg(short = 's', long, default_value_t = 1)]
    pub saddr: u32,

    #[arg(short = 'd', long, default_value_t = 100)]
    pub daddr: u32,

    #[arg(short = 'i', long, default_value_t = 0)]
    pub id: u32,

    #[arg(short = 'b', long, default_value_t = String::from("127.0.0.1:21010"))]
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

    let n = args.id + args.loop_times;

    let start = time::Instant::now();
    for i in args.id..n {
        // 送信するJSONデータ
        let json_data = json!({
            "msg_type": args.msg_type,
            "saddr": args.saddr,
            "daddr": args.daddr,
            "id": i,
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
        let n = stream.read(&mut buffer)?;

    }
    let elapsed = start.elapsed();
    println!("Elapsed: {}.{:03} seconds", elapsed.as_secs(), elapsed.subsec_millis());

    Ok(())
}
