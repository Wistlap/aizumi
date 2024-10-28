use std::{io::{Read, Write}, net::TcpStream};
use std::time;

pub use clap::Parser;
use std::fmt::Display;

use aizumi::messaging::{Request, Response, MsgType};

/// Command-line Argument of m-broker-rs
#[derive(Parser, Debug)]
#[command(author, version, about)]
pub struct Args {

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
        let output = format!("saddr: {}\n", self.saddr);
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

    // Requestを作成
    let req = Request::new(
        MsgType::MSG_HELO_REQ,
        args.saddr as i32,
        args.daddr as i32,
        0_i32,
        String::from("hello")
    );

    // TODO: Request 構造体のメソッドとして to_bytes() を実装すべきか．
    // req を &[u8] に変換
    let raw_req = bincode::serialize(&req).unwrap();
    let mut formatted_req:[u8; 1024] = [0; 1024];
    formatted_req[..raw_req.len()].copy_from_slice(&raw_req);

    // Request を送信
    let res = stream.write(&formatted_req);
    if let Err(e) = res {
        eprintln!("Failed to send data: {}", e);
        return Err(e);
    }
    stream.flush()?;

    // サーバからのレスポンスを受信
    let mut buffer = [0; 1024];
    let _n = stream.read(&mut buffer)?;
    // 受信したメッセージを Response 構造体にデシリアライズ
    let _res: Response = bincode::deserialize(&buffer).unwrap();
    // println!("{:?}", res);

    let n = args.id + args.loop_times;

    let start = time::Instant::now();
    for _i in args.id..n {

        // サーバからのリクエストを受信
        let _n = stream.read(&mut buffer)?;
        // 受信したメッセージを Response 構造体にデシリアライズ
        let _msg: Response = bincode::deserialize(&buffer).unwrap();
        // println!("{:?}", _msg);

        // Requestを作成
        let req = Request::new(
            MsgType::MSG_PUSH_ACK,
            args.saddr as i32,
            args.daddr as i32,
            0_i32,
            String::from("")
        );

        let raw_req = bincode::serialize(&req).unwrap();
        let mut formatted_req:[u8; 1024] = [0; 1024];
        formatted_req[..raw_req.len()].copy_from_slice(&raw_req);

        let res = stream.write(&formatted_req);
        if let Err(e) = res {
            eprintln!("Failed to send data: {}", e);
            return Err(e);
        }
        stream.flush()?;
    }
    let elapsed = start.elapsed();
    println!("(receiver) Elapsed: {}.{:03} seconds", elapsed.as_secs(), elapsed.subsec_millis());

    Ok(())
}
