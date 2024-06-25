use aipim::client::Client;
use log::LevelFilter;
use simplelog::{ColorChoice, CombinedLogger, Config, TermLogger, TerminalMode};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();

    CombinedLogger::init(vec![TermLogger::new(
        LevelFilter::Debug,
        Config::default(),
        TerminalMode::Mixed,
        ColorChoice::Auto,
    )])
    .unwrap();

    let var_name = Client::new("gpt-4o");
    let cli = var_name?;
    let response = cli.message().text("Why is the sky red?").send().await?;
    log::debug!("Response:\n{}", response.text);

    Ok(())
}
