pub use clap::Parser;
use std::fmt::Display;

/// Command-line Argument of m-broker-rs
#[derive(Parser, Debug)]
#[command(author, version, about)]
pub struct Args {
    /// Debug level (0 ~ 5)
    ///
    /// If 0 is set
    #[arg(short = 'd', long, default_value_t = 2)]
    pub log_level: u32,

    /// The set of IP address and port that the broker binds
    ///
    /// Example: 'localhost:5555', '127.0.0.1:8080'
    #[arg(short = 'b', long, default_value_t = String::from("127.0.0.1:5555"))]
    pub address: String,

    /// Timeout (ms) when the broker waits for 'epoll_wait'
    ///
    /// If '0' is set, 'epoll_wait' returns immediately regardless of packet arrival\n
    /// If '-1' is set, 'epoll_wait' continues to block until a packet arrives
    /// Ignored if --thread-pool is set
    #[arg(short = 't', long, default_value_t = 1)]
    pub timeout: u32,

    /// The number of threads the broker creates
    ///
    /// Ignored if --thread-pool is not set
    #[arg(short = 'n', long, default_value_t = 10)]
    pub threads: u32,

    /// Ignored
    #[arg(short = 'c', long, default_value_t = 1000)]
    pub msgs: i32,

    /// "m-sender01.log"
    #[arg(short = 'l', long, default_value_t = String::new())]
    pub log_file: String,

    /// "m-sender01.pid"
    #[arg(short = 'p', long, default_value_t = String::new())]
    pub pid_file: String,

    /// An integer repressing the broker id
    ///
    /// This must be unique toward other senders and receivers
    #[arg(short = 'u', long, default_value_t = 0)]
    pub myid: u32,
}

impl Display for Args {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let output = format!("debug_level: {}\n", self.log_level);
        let output = format!("{output}address: {}\n", self.address);
        let output = format!("{output}timeout: {}\n", self.timeout);
        let output = format!("{output}threads: {}\n", self.threads);
        let output = format!("{output}msgs: {}\n", self.msgs);
        let output = format!("{output}log-file: {}\n", self.log_file);
        let output = format!("{output}pid-file: {}\n", self.pid_file);
        let output = format!("{output}myid: {}\n", self.myid);

        f.write_str(output.as_str())
    }
}
