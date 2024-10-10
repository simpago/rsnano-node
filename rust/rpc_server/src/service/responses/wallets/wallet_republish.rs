use rsnano_core::{Account, BlockEnum, BlockHash, WalletId};
use rsnano_node::{Node, NodeExt};
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
