use rsnano_core::{Account, BlockEnum, BlockHash, WalletId};
use rsnano_node::node::{Node, NodeExt};
use rsnano_rpc_messages::{BlockHashesDto, ErrorDto};
use serde_json::to_string_pretty;
use std::collections::VecDeque;
use std::{sync::Arc, time::Duration};

pub async fn wallet_republish(
    node: Arc<Node>,
    enable_control: bool,
    wallet: WalletId,
    count: u64,
) -> String {
    if !enable_control {
        return to_string_pretty(&ErrorDto::new("RPC control is disabled".to_string())).unwrap();
    }

    let accounts = match node.wallets.get_accounts_of_wallet(&wallet) {
        Ok(accounts) => accounts,
        Err(e) => return to_string_pretty(&ErrorDto::new(e.to_string())).unwrap(),
    };

    let (blocks, republish_bundle) = collect_blocks_to_republish(node.clone(), accounts, count);
    node.flood_block_many(
        republish_bundle.into(),
        Box::new(|| ()),
        Duration::from_millis(25),
    );
    to_string_pretty(&BlockHashesDto::new(blocks)).unwrap()
}

fn collect_blocks_to_republish(
    node: Arc<Node>,
    accounts: Vec<Account>,
    count: u64,
) -> (Vec<BlockHash>, VecDeque<BlockEnum>) {
    let mut blocks = Vec::new();
    let mut republish_bundle = VecDeque::new();
    let tx = node.ledger.read_txn();

    for account in accounts {
        let mut latest = node.ledger.any().account_head(&tx, &account).unwrap();
        let mut hashes = Vec::new();

        while !latest.is_zero() && hashes.len() < count as usize {
            hashes.push(latest);
            if let Some(block) = node.ledger.get_block(&tx, &latest) {
                latest = block.previous();
            } else {
                latest = BlockHash::zero();
            }
        }

        for hash in hashes.into_iter().rev() {
            if let Some(block) = node.ledger.get_block(&tx, &hash) {
                republish_bundle.push_back(block);
                blocks.push(hash);
            }
        }
    }

    (blocks, republish_bundle)
}

#[cfg(test)]
mod tests {
    use rsnano_core::{Amount, BlockEnum, StateBlock, WalletId, DEV_GENESIS_KEY};
    use rsnano_ledger::{DEV_GENESIS_ACCOUNT, DEV_GENESIS_HASH, DEV_GENESIS_PUB_KEY};
    use rsnano_node::{node::Node, wallets::WalletsExt};
    use std::{sync::Arc, time::Duration};
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
    fn wallet_republish() {
        let mut system = System::new();
        let node = system.make_node();

        let send = send_block(node.clone());

        let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), true);

        let wallet = WalletId::zero();

        node.wallets.create(wallet);

        node.wallets
            .insert_adhoc2(&wallet, &DEV_GENESIS_KEY.private_key(), false)
            .unwrap();

        let result = node
            .tokio
            .block_on(async { rpc_client.wallet_republish(wallet, 1).await.unwrap() });

        assert!(
            result.blocks.len() == 1,
            "Expected 1 block, got {}",
            result.blocks.len()
        );
        assert_eq!(result.blocks[0], send.hash(), "Unexpected block hash");

        server.abort();
    }

    #[test]
    fn wallet_republish_fails_without_enable_control() {
        let mut system = System::new();
        let node = system.make_node();

        let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), false);

        let result = node
            .tokio
            .block_on(async { rpc_client.wallet_republish(WalletId::zero(), 1).await });

        assert_eq!(
            result.err().map(|e| e.to_string()),
            Some("node returned error: \"RPC control is disabled\"".to_string())
        );

        server.abort();
    }

    #[test]
    fn wallet_republish_fails_with_wallet_not_found() {
        let mut system = System::new();
        let node = system.make_node();

        let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), true);

        let result = node
            .tokio
            .block_on(async { rpc_client.wallet_republish(WalletId::zero(), 1).await });

        assert_eq!(
            result.err().map(|e| e.to_string()),
            Some("node returned error: \"Wallet not found\"".to_string())
        );

        server.abort();
    }
}
