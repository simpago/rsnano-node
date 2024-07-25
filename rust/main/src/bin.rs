use anyhow::Result;
use clap::Parser;
use cli::Cli;

mod cli;

fn main() -> Result<()> {
    let cli = Cli::parse();
    cli.run()?;

    Ok(())
}
