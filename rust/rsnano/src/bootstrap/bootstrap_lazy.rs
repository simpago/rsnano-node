use crate::{block_processing::BlockProcessor, ledger::Ledger, utils::Logger, websocket::Listener};
use anyhow::Result;
use std::sync::{Arc, Weak};

use super::{BootstrapAttempt, BootstrapInitiator, BootstrapMode};

pub struct BootstrapAttemptLazy {
    pub attempt: BootstrapAttempt,
}

impl BootstrapAttemptLazy {
    pub fn new(
        logger: Arc<dyn Logger>,
        websocket_server: Arc<dyn Listener>,
        block_processor: Weak<BlockProcessor>,
        bootstrap_initiator: Weak<BootstrapInitiator>,
        ledger: Arc<Ledger>,
        id: &str,
        incremental_id: u64,
    ) -> Result<Self> {
        Ok(Self {
            attempt: BootstrapAttempt::new(
                logger,
                websocket_server,
                block_processor,
                bootstrap_initiator,
                ledger,
                id,
                BootstrapMode::Lazy,
                incremental_id,
            )?,
        })
    }
}
