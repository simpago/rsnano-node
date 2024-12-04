use anyhow::Result;
use clap::Parser;
use cli::{Cli, CliInfrastructure};

mod cli;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let mut infra = CliInfrastructure::default();
    cli.run(&mut infra).await?;
    Ok(())
}
