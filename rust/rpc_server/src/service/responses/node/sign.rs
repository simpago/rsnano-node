use rsnano_core::{sign_message, utils::MemoryStream, BlockEnum, RawKey};
use rsnano_node::Node;
use rsnano_rpc_messages::{ErrorDto, SignArgs, SignDto};
use serde_json::to_string_pretty;
use std::sync::Arc;

pub async fn sign(node: Arc<Node>, args: SignArgs) -> String {
    let block: BlockEnum = args.block.into();

    let prv = if let Some(key) = args.key {
        key
    } else if let (Some(wallet), Some(account)) = (args.wallet, args.account) {
        node.wallets.fetch(&wallet, &account.into()).unwrap()
    } else {
        return to_string_pretty(&ErrorDto::new("Block create key required".to_string())).unwrap();
    };

    let signature = if prv != RawKey::zero() {
        let mut stream = MemoryStream::new();
        block.serialize(&mut stream);

        let signature = sign_message(&prv, &stream.to_vec());
        signature
    } else {
        return to_string_pretty(&ErrorDto::new("Block create key required".to_string())).unwrap();
    };

    let sign_dto = SignDto::new(signature, block.json_representation());

    to_string_pretty(&sign_dto).unwrap()
}
