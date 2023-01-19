use rsnano_core::{Account, BlockHash};

mod block_cementor;
mod cement_queue;
mod confirmation_height_unbounded;

pub use confirmation_height_unbounded::ConfirmationHeightUnbounded;

#[derive(Clone, Debug)]
pub struct ConfHeightDetails {
    pub account: Account,
    pub hash: BlockHash,
    pub new_height: u64,
    pub num_blocks_confirmed: u64,
    pub block_callback_data: Vec<BlockHash>,
    pub source_block_callback_data: Vec<BlockHash>,
}

#[derive(Clone)]
pub struct ConfirmedIteratedPair {
    pub confirmed_height: u64,
    pub iterated_height: u64,
}
