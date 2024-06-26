use std::net::SocketAddr;

use clap::Parser;
use log::LevelFilter;
use simplelog::{ColorChoice, CombinedLogger, Config, TermLogger, TerminalMode};

mod api;

#[derive(Parser)]
#[command(version, about)]
struct Cli {
    /// Address for the server to listen on.
    #[arg(short, long, default_value_t = default_address())]
    address: SocketAddr,

    /// Name of the default model to use.
    #[arg(short = 'm', long)]
    default_model: String,

    /// Verbose mode, display debug information.
    #[arg(short, long)]
    verbose: bool,
}

fn default_address() -> SocketAddr {
    "127.0.0.1:3000".parse().unwrap()
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let filter = if cli.verbose {
        LevelFilter::Debug
    } else {
        LevelFilter::Info
    };

    CombinedLogger::init(vec![
        TermLogger::new(
            filter,
            Config::default(),
            TerminalMode::Mixed,
            ColorChoice::Auto,
        ),
        // WriteLogger::new(LevelFilter::Info, Config::default(), File::create("my_rust_binary.log").unwrap()),
    ])
    .unwrap();

    api::listen(cli.address, cli.default_model).await?;
    Ok(())
}
