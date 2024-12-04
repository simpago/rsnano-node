mod legacy_change_block_builder;
mod legacy_open_block_builder;
mod legacy_receive_block_builder;
mod legacy_send_block_builder;
mod saved_account_chain;
mod state_block_builder;

pub use legacy_change_block_builder::LegacyChangeBlockBuilder;
pub use legacy_open_block_builder::LegacyOpenBlockBuilder;
pub use legacy_receive_block_builder::LegacyReceiveBlockBuilder;
pub use legacy_send_block_builder::LegacySendBlockBuilder;
pub use saved_account_chain::SavedAccountChain;
pub use state_block_builder::StateBlockBuilder;

pub struct BlockBuilder {}

impl BlockBuilder {
    pub fn state() -> StateBlockBuilder {
        StateBlockBuilder::new()
    }

    pub fn legacy_open() -> LegacyOpenBlockBuilder {
        LegacyOpenBlockBuilder::new()
    }

    pub fn legacy_receive() -> LegacyReceiveBlockBuilder {
        LegacyReceiveBlockBuilder::new()
    }

    pub fn legacy_send() -> LegacySendBlockBuilder {
        LegacySendBlockBuilder::new()
    }

    pub fn legacy_change() -> LegacyChangeBlockBuilder {
        LegacyChangeBlockBuilder::new()
    }
}
