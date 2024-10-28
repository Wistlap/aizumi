use aizumi::start_raft_node;
use clap::Parser;
use tracing_subscriber::EnvFilter;

const MY_ID: u64 = 5000;

#[derive(Parser, Clone, Debug)]
#[clap(author, version, about)]
pub struct Opt {
    #[arg(short, long, default_value_t = MY_ID)]
    pub id: u64,

    #[arg(short, long, default_value = "127.0.0.1:8080")]
    pub addr: String,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Setup the logger
    tracing_subscriber::fmt()
        .with_target(true)
        .with_thread_ids(true)
        .with_level(true)
        .with_ansi(false)
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    // Parse the parameters passed by arguments.
    let options = Opt::parse();

    start_raft_node(options.id, options.addr).await
}
