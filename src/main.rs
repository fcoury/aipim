use client::Client;
use simplelog::{ColorChoice, CombinedLogger, Config, LevelFilter, TermLogger, TerminalMode};

mod client;
mod provider;

const MODEL: &str = "claude-3-5-sonnet-20240620";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();

    CombinedLogger::init(vec![
        TermLogger::new(
            LevelFilter::Trace,
            Config::default(),
            TerminalMode::Mixed,
            ColorChoice::Auto,
        ),
        // WriteLogger::new(
        //     LevelFilter::Info,
        //     Config::default(),
        //     File::create("my_rust_binary.log").unwrap(),
        // ),
    ])
    .unwrap();

    let cli = Client::new(MODEL);
    let response = cli
        .message()
        .prompt("blank_form")?
        .image_file("form-felipe.jpg")?
        .send()
        .await?;
    println!("Response:\n{}", response.text);

    Ok(())
}
