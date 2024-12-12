/// Module that parses and stores the command-line arguments
mod args;
/// Module that performs message broker
mod mbroker;

use args::{Args, Parser};
use mbroker::MBroker;

// use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// Entry point for m-broker-rs
///
/// Command-line argument is set in [`Args`] struct
// #[tokio::main]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    MBroker::new(
        args.myid,
        args.address,
        args.timeout,
        args.log_file,
        args.log_level,
        args.pid_file,
        args.raft_nodes,
    )?
    .run().unwrap();

    Ok(())
}
