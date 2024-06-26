mod api;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    api::listen(([0, 0, 0, 0], 3000).into(), "gpt-4o").await?;
    Ok(())
}
