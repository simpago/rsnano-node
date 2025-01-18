use crate::command_handler::RpcCommandHandler;
use anyhow::anyhow;
use rsnano_core::{Block, BlockHash, PendingKey};
use rsnano_rpc_messages::{BlockHashesResponse, RepublishArgs};
use std::time::Duration;

impl RpcCommandHandler {
    pub(crate) fn republish(&self, args: RepublishArgs) -> anyhow::Result<BlockHashesResponse> {
        let count = args.count.unwrap_or(1024.into()).inner();
        let sources = args.sources.unwrap_or_default().inner();
        let destinations = args.destinations.unwrap_or_default().inner();
        let mut hash = args.hash;
        let mut blocks = Vec::new();
        let tx = self.node.store.tx_begin_read();

        let mut republish_bundle: Vec<Block> = Vec::new();

        for _ in 0..count {
            if hash.is_zero() {
                break;
            }

            let block = self.load_block_any(&tx, &hash)?;

            // Republish source chain
            if sources != 0 {
                let mut source = block.source_or_link();
                let mut hashes = Vec::new();
                let mut block_a = self.load_block_any(&tx, &source);

                while let Ok(b) = block_a {
                    if hashes.len() >= sources {
                        break;
                    }
                    hashes.push(source);
                    source = b.previous();
                    block_a = self.load_block_any(&tx, &source);
                }

                for hash in hashes.into_iter().rev() {
                    if let Some(b) = self.node.ledger.any().get_block(&tx, &hash) {
                        republish_bundle.push(b.into());
                        blocks.push(hash);
                    }
                }
            }

            // Republish block
            republish_bundle.push(block.into());
            blocks.push(hash);

            // Republish destination chain
            if destinations != 0 {
                let block_b = self.load_block_any(&tx, &hash)?;
                if let Some(destination) = block_b.destination() {
                    if self
                        .node
                        .ledger
                        .any()
                        .get_pending(&tx, &PendingKey::new(destination, hash))
                        .is_none()
                    {
                        let mut previous = self
                            .node
                            .ledger
                            .any()
                            .account_head(&tx, &destination)
                            .ok_or_else(|| anyhow!("Account head not found"))?;

                        let mut dest_block = self.node.ledger.any().get_block(&tx, &previous);
                        let mut dest_hashes = Vec::new();
                        let mut source = BlockHash::zero();

                        while let Some(db) = dest_block {
                            if hash == source {
                                break;
                            }
                            dest_hashes.push(previous);
                            source = db
                                .source_field()
                                .or_else(|| {
                                    if db.is_send() {
                                        None
                                    } else {
                                        db.link_field().map(|link| link.into())
                                    }
                                })
                                .unwrap_or_default();
                            previous = db.previous();
                            dest_block = self.node.ledger.any().get_block(&tx, &previous);
                        }

                        for hash in dest_hashes.iter().rev().take(destinations) {
                            if let Some(b) = self.node.ledger.any().get_block(&tx, &hash) {
                                republish_bundle.push(b.into());
                                blocks.push(*hash);
                            }
                        }
                    }
                }
            }

            // Move to the next block
            hash = self
                .node
                .ledger
                .any()
                .block_successor(&tx, &hash)
                .unwrap_or_default();
        }

        // Flood the network with republished blocks
        self.node.flood_block_many(
            republish_bundle.into(),
            Box::new(|| {}),
            Duration::from_millis(25),
        );

        Ok(BlockHashesResponse::new(blocks))
    }
}
