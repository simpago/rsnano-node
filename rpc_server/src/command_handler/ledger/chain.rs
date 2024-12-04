use crate::command_handler::RpcCommandHandler;
use rsnano_core::BlockHash;
use rsnano_rpc_messages::{
    unwrap_bool_or_false, unwrap_u64_or_zero, BlockHashesResponse, ChainArgs,
};

impl RpcCommandHandler {
    pub(crate) fn chain(&self, args: ChainArgs, successors: bool) -> BlockHashesResponse {
        let successors = successors != unwrap_bool_or_false(args.reverse);
        let mut hash = args.block;
        let count: u64 = args.count.into();
        let mut offset = unwrap_u64_or_zero(args.offset);
        let mut blocks = Vec::new();

        let txn = self.node.store.tx_begin_read();

        while !hash.is_zero() && blocks.len() < count as usize {
            if let Some(block) = self.node.ledger.any().get_block(&txn, &hash) {
                if offset > 0 {
                    offset -= 1;
                } else {
                    blocks.push(hash);
                }

                hash = if successors {
                    self.node
                        .ledger
                        .any()
                        .block_successor(&txn, &hash)
                        .unwrap_or_else(BlockHash::zero)
                } else {
                    block.previous()
                };
            } else {
                hash = BlockHash::zero();
            }
        }

        BlockHashesResponse::new(blocks)
    }
}
