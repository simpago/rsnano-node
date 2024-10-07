use rsnano_core::BlockEnum;
use rsnano_ledger::BlockStatus;
use rsnano_node::node::Node;
use rsnano_rpc_messages::{BlockHashRpcMessage, ErrorDto, ProcessArgs};
use serde_json::to_string_pretty;
use std::sync::Arc;

pub async fn process(node: Arc<Node>, args: ProcessArgs) -> String {
    let is_async = args.is_async.unwrap_or(false);
    let block: BlockEnum = args.block.into();

    if node
        .network_params
        .work
        .validate_entry(block.work_version(), &block.root(), block.work())
    {
        return to_string_pretty(&ErrorDto::new("Work low".to_string())).unwrap();
    }

    if !is_async {
        match node.process_local(block.clone()) {
            Some(result) => match result {
                BlockStatus::Progress => {
                    let hash = block.hash();
                    to_string_pretty(&BlockHashRpcMessage::new("hash".to_string(), hash)).unwrap()
                }
                BlockStatus::GapPrevious => {
                    to_string_pretty(&ErrorDto::new("gap previous".to_string())).unwrap()
                }
                BlockStatus::GapSource => {
                    to_string_pretty(&ErrorDto::new("Gap source".to_string())).unwrap()
                }
                BlockStatus::Old => to_string_pretty(&ErrorDto::new("Old".to_string())).unwrap(),
                BlockStatus::BadSignature => {
                    to_string_pretty(&ErrorDto::new("Bad signature".to_string())).unwrap()
                }
                BlockStatus::NegativeSpend => {
                    to_string_pretty(&ErrorDto::new("Negative spend".to_string())).unwrap()
                }
                BlockStatus::BalanceMismatch => {
                    to_string_pretty(&ErrorDto::new("Balance mismatch".to_string())).unwrap()
                }
                BlockStatus::Unreceivable => {
                    to_string_pretty(&ErrorDto::new("Unreceivable".to_string())).unwrap()
                }
                BlockStatus::BlockPosition => {
                    to_string_pretty(&ErrorDto::new("Block position".to_string())).unwrap()
                }
                BlockStatus::GapEpochOpenPending => {
                    to_string_pretty(&ErrorDto::new("Gap epoch open pending".to_string())).unwrap()
                }
                BlockStatus::Fork => {
                    if args.force.unwrap_or(false) {
                        node.active.erase(&block.qualified_root());
                        node.block_processor.force(Arc::new(block.clone()));
                        let hash = block.hash();
                        to_string_pretty(&BlockHashRpcMessage::new("hash".to_string(), hash))
                            .unwrap()
                    } else {
                        to_string_pretty(&ErrorDto::new("Fork".to_string())).unwrap()
                    }
                }
                BlockStatus::InsufficientWork => {
                    to_string_pretty(&ErrorDto::new("Insufficient work".to_string())).unwrap()
                }
                BlockStatus::OpenedBurnAccount => {
                    to_string_pretty(&ErrorDto::new("Opened burn account".to_string())).unwrap()
                }
                _ => to_string_pretty(&ErrorDto::new("Other".to_string())).unwrap(),
            },
            None => to_string_pretty(&ErrorDto::new("Stopped".to_string())).unwrap(),
        }
    } else {
        if let BlockEnum::State(_) = block {
            node.process(block);
            to_string_pretty(&serde_json::json!({"started": "1"})).unwrap_or_default()
        } else {
            to_string_pretty(&ErrorDto::new("Is not state block".to_string())).unwrap()
        }
    }
}

#[cfg(test)]
mod tests {
    use rsnano_core::{Amount, BlockEnum, BlockSubType, StateBlock, DEV_GENESIS_KEY};
    use rsnano_ledger::{DEV_GENESIS_ACCOUNT, DEV_GENESIS_HASH, DEV_GENESIS_PUB_KEY};
    use test_helpers::{setup_rpc_client_and_server, System};

    #[test]
    fn process() {
        let mut system = System::new();
        let node = system.make_node();

        let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), false);

        let send1 = BlockEnum::State(StateBlock::new(
            *DEV_GENESIS_ACCOUNT,
            *DEV_GENESIS_HASH,
            *DEV_GENESIS_PUB_KEY,
            Amount::MAX - Amount::raw(100),
            DEV_GENESIS_KEY.account().into(),
            &DEV_GENESIS_KEY,
            node.work_generate_dev((*DEV_GENESIS_HASH).into()),
        ));

        let result = node.tokio.block_on(async {
            rpc_client
                .process(
                    Some(BlockSubType::Send),
                    send1.json_representation(),
                    None,
                    None,
                    None,
                )
                .await
                .unwrap()
        });

        assert_eq!(result.value, send1.hash());

        assert_eq!(node.latest(&*DEV_GENESIS_ACCOUNT), send1.hash());

        server.abort();
    }

    #[test]
    fn process_fails_with_low_work() {
        let mut system = System::new();
        let node = system.make_node();

        let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), false);

        let send1 = BlockEnum::State(StateBlock::new(
            *DEV_GENESIS_ACCOUNT,
            *DEV_GENESIS_HASH,
            *DEV_GENESIS_PUB_KEY,
            Amount::MAX - Amount::raw(100),
            DEV_GENESIS_KEY.account().into(),
            &DEV_GENESIS_KEY,
            1,
        ));

        let result = node.tokio.block_on(async {
            rpc_client
                .process(
                    Some(BlockSubType::Send),
                    send1.json_representation(),
                    None,
                    None,
                    None,
                )
                .await
        });

        assert_eq!(
            result.err().map(|e| e.to_string()),
            Some("node returned error: \"Work low\"".to_string())
        );

        server.abort();
    }
}
