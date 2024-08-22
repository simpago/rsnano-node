mod block_factory;
mod load_test;
mod program_args;
mod test_node;

use anyhow::Result;
pub use block_factory::*;
use load_test::*;
pub use program_args::*;
use rsnano_node::config::force_nano_dev_network;
pub use test_node::*;

#[tokio::main]
async fn main() -> Result<()> {
    force_nano_dev_network();
    let args = ProgramArgs::parse()?;
    args.validate_paths()?;
    LoadTest::new(args).run().await
}
