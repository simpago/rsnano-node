use crate::command_handler::RpcCommandHandler;
use anyhow::bail;
use rsnano_core::{BlockHash, BlockType, PendingKey};
use rsnano_rpc_messages::{
    unwrap_bool_or_false, BlockInfoResponse, BlocksInfoArgs, BlocksInfoResponse,
};
use std::collections::HashMap;

impl RpcCommandHandler {
    pub(crate) fn blocks_info(&self, args: BlocksInfoArgs) -> anyhow::Result<BlocksInfoResponse> {
        let receivable = unwrap_bool_or_false(args.receivable);
        let receive_hash = unwrap_bool_or_false(args.receive_hash);
        let source = unwrap_bool_or_false(args.source);
        let include_not_found = unwrap_bool_or_false(args.include_not_found);

        let txn = self.node.ledger.read_txn();
        let mut blocks: HashMap<BlockHash, BlockInfoResponse> = HashMap::new();
        let mut blocks_not_found = Vec::new();

        for hash in args.hashes {
            if let Some(block) = self.node.ledger.any().get_block(&txn, &hash) {
                let block_account = block.account();
                let amount = self.node.ledger.any().block_amount(&txn, &hash);
                let balance = self.node.ledger.any().block_balance(&txn, &hash).unwrap();
                let height = block.height();
                let local_timestamp = block.timestamp();
                let successor = block.successor().unwrap_or_default();
                let confirmed = self
                    .node
                    .ledger
                    .confirmed()
                    .block_exists_or_pruned(&txn, &hash);
                let contents = block.json_representation();

                let subtype = if block.block_type() == BlockType::State {
                    Some(block.subtype().into())
                } else {
                    None
                };

                let mut block_info = BlockInfoResponse {
                    block_account,
                    amount,
                    balance,
                    height: height.into(),
                    local_timestamp: local_timestamp.as_u64().into(),
                    successor,
                    confirmed: confirmed.into(),
                    contents,
                    subtype,
                    receivable: None,
                    receive_hash: None,
                    source_account: None,
                };

                if receivable || receive_hash {
                    if !block.is_send() {
                        if receivable {
                            block_info.receivable = Some(0.into());
                        }
                        if receive_hash {
                            block_info.receive_hash = Some(BlockHash::zero());
                        }
                    } else if self
                        .node
                        .ledger
                        .any()
                        .get_pending(&txn, &PendingKey::new(block.destination_or_link(), hash))
                        .is_some()
                    {
                        if receivable {
                            block_info.receivable = Some(1.into())
                        }
                        if receive_hash {
                            block_info.receive_hash = Some(BlockHash::zero());
                        }
                    } else {
                        if receivable {
                            block_info.receivable = Some(0.into());
                        }
                        if receive_hash {
                            let receive_block = self.node.ledger.find_receive_block_by_send_hash(
                                &txn,
                                &block.destination_or_link(),
                                &hash,
                            );

                            block_info.receive_hash = Some(match receive_block {
                                Some(b) => b.hash(),
                                None => BlockHash::zero(),
                            });
                        }
                    }
                }

                if source {
                    if !block.is_receive()
                        || !self
                            .node
                            .ledger
                            .any()
                            .block_exists(&txn, &block.source_or_link())
                    {
                        block_info.source_account = Some("0".to_string());
                    } else {
                        let block_a = self
                            .node
                            .ledger
                            .any()
                            .get_block(&txn, &block.source_or_link())
                            .unwrap();
                        block_info.source_account = Some(block_a.account().encode_account());
                    }
                }

                blocks.insert(hash, block_info);
            } else if include_not_found {
                blocks_not_found.push(hash);
            } else {
                bail!(Self::BLOCK_NOT_FOUND);
            }
        }

        Ok(BlocksInfoResponse {
            blocks,
            blocks_not_found: if include_not_found {
                Some(blocks_not_found)
            } else {
                None
            },
        })
    }
}
