use anyhow::Result;
use clap::Parser;
use cli::Cli;

mod cli;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    cli.run().await?;

    Ok(())
}
