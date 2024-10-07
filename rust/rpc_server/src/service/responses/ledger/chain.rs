use rsnano_core::BlockHash;
use rsnano_node::node::Node;
use rsnano_rpc_messages::{BlockHashesDto, ChainArgs};
use serde_json::to_string_pretty;
use std::sync::Arc;

pub async fn chain(node: Arc<Node>, args: ChainArgs, successors: bool) -> String {
    let successors = successors != args.reverse.unwrap_or(false);
    let mut hash = args.block;
    let count = args.count;
    let mut offset = args.offset.unwrap_or(0);
    let mut blocks = Vec::new();

    let txn = node.store.tx_begin_read();

    while !hash.is_zero() && blocks.len() < count as usize {
        if let Some(block) = node.ledger.any().get_block(&txn, &hash) {
            if offset > 0 {
                offset -= 1;
            } else {
                blocks.push(hash);
            }

            hash = if successors {
                node.ledger
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

    to_string_pretty(&BlockHashesDto::new(blocks)).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rsnano_core::{Amount, BlockEnum, StateBlock, DEV_GENESIS_KEY};
    use rsnano_ledger::{DEV_GENESIS_ACCOUNT, DEV_GENESIS_HASH, DEV_GENESIS_PUB_KEY};
    use std::time::Duration;
    use test_helpers::{assert_timely_msg, setup_rpc_client_and_server, System};

    fn send_block(node: Arc<Node>) -> BlockEnum {
        let send1 = BlockEnum::State(StateBlock::new(
            *DEV_GENESIS_ACCOUNT,
            *DEV_GENESIS_HASH,
            *DEV_GENESIS_PUB_KEY,
            Amount::MAX - Amount::raw(1),
            DEV_GENESIS_KEY.account().into(),
            &DEV_GENESIS_KEY,
            node.work_generate_dev((*DEV_GENESIS_HASH).into()),
        ));

        node.process_active(send1.clone());
        assert_timely_msg(
            Duration::from_secs(5),
            || node.active.active(&send1),
            "not active on node 1",
        );

        send1
    }

    #[test]
    fn chain() {
        let mut system = System::new();
        let node = system.make_node();

        let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), true);

        let block = send_block(node.clone());

        let result = node.tokio.block_on(async {
            rpc_client
                .chain(block.hash(), u64::MAX, None, None)
                .await
                .unwrap()
        });

        let blocks = result.blocks;

        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0], block.hash());
        assert_eq!(blocks[1], *DEV_GENESIS_HASH);

        server.abort();
    }
}
