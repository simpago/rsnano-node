use rsnano_core::{sign_message, utils::MemoryStream, BlockEnum, RawKey};
use rsnano_node::node::Node;
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

#[cfg(test)]
mod tests {
    use rsnano_core::PublicKey;
    use rsnano_core::{
        validate_block_signature, Account, Amount, BlockEnum, StateBlock, WalletId, DEV_GENESIS_KEY,
    };
    use rsnano_ledger::{DEV_GENESIS_ACCOUNT, DEV_GENESIS_HASH, DEV_GENESIS_PUB_KEY};
    use rsnano_node::wallets::WalletsExt;
    use test_helpers::{setup_rpc_client_and_server, System};

    #[test]
    fn sign() {
        let mut system = System::new();
        let node = system.make_node();

        let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), false);

        let key = rsnano_core::KeyPair::new();

        let wallet_id = WalletId::zero();
        node.wallets.create(wallet_id);
        node.wallets
            .insert_adhoc2(&wallet_id, &key.private_key(), false)
            .unwrap();

        let send = BlockEnum::State(StateBlock::new(
            *DEV_GENESIS_ACCOUNT,
            *DEV_GENESIS_HASH,
            *DEV_GENESIS_PUB_KEY,
            Amount::MAX - Amount::raw(1_000_000), // Equivalent to nano::Gxrb_ratio
            Account::from(key.public_key()).into(),
            &DEV_GENESIS_KEY,
            node.work_generate_dev((*DEV_GENESIS_HASH).into()),
        ));

        let result = node.tokio.block_on(async {
            rpc_client
                .sign(
                    None,
                    Some(wallet_id),
                    Some(key.public_key().into()),
                    send.json_representation(),
                )
                .await
                .unwrap()
        });

        let signed_block: BlockEnum = result.block.into();

        if let BlockEnum::State(ref state_block) = signed_block {
            assert!(validate_block_signature(&state_block).is_ok());
        } else {
            panic!("Expected a state block");
        }

        assert_eq!(signed_block.block_signature(), send.block_signature());

        assert_eq!(signed_block.hash(), send.hash());

        server.abort();
    }

    #[test]
    fn sign_without_key() {
        let mut system = System::new();
        let node = system.make_node();

        let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), false);

        let send = BlockEnum::State(StateBlock::new(
            *DEV_GENESIS_ACCOUNT,
            *DEV_GENESIS_HASH,
            *DEV_GENESIS_PUB_KEY,
            Amount::MAX - Amount::raw(1_000_000),
            Account::from(PublicKey::zero()).into(),
            &DEV_GENESIS_KEY,
            node.work_generate_dev((*DEV_GENESIS_HASH).into()),
        ));

        let result = node.tokio.block_on(async {
            rpc_client
                .sign(None, None, None, send.json_representation())
                .await
        });

        assert_eq!(
            result.err().map(|e| e.to_string()),
            Some("node returned error: \"Block create key required\"".to_string())
        );

        server.abort();
    }
}
