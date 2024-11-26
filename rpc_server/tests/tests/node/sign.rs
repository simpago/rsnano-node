use rsnano_core::PublicKey;
use rsnano_core::{
    validate_block_signature, Account, Amount, Block, StateBlock, WalletId, DEV_GENESIS_KEY,
};
use rsnano_ledger::{DEV_GENESIS_ACCOUNT, DEV_GENESIS_HASH, DEV_GENESIS_PUB_KEY};
use rsnano_node::wallets::WalletsExt;
use rsnano_rpc_messages::SignArgs;
use test_helpers::{setup_rpc_client_and_server, System};

#[test]
fn sign() {
    let mut system = System::new();
    let node = system.make_node();

    let server = setup_rpc_client_and_server(node.clone(), false);

    let key = rsnano_core::KeyPair::new();

    let wallet_id = WalletId::zero();
    node.wallets.create(wallet_id);
    node.wallets
        .insert_adhoc2(&wallet_id, &key.private_key(), false)
        .unwrap();

    let send = Block::State(StateBlock::new(
        *DEV_GENESIS_ACCOUNT,
        *DEV_GENESIS_HASH,
        *DEV_GENESIS_PUB_KEY,
        Amount::MAX - Amount::raw(1_000_000),
        Account::from(key.public_key()).into(),
        &DEV_GENESIS_KEY,
        node.work_generate_dev(*DEV_GENESIS_HASH),
    ));

    let args: SignArgs = SignArgs {
        block: Some(send.json_representation()),
        wallet: None,
        account: None,
        hash: None,
        key: Some(DEV_GENESIS_KEY.private_key()),
    };

    let result = node
        .runtime
        .block_on(async { server.client.sign(args).await.unwrap() });

    let signed_block: Block = result.block.unwrap().into();

    if let Block::State(ref state_block) = signed_block {
        assert!(validate_block_signature(&state_block).is_ok());
    } else {
        panic!("Expected a state block");
    }

    assert_eq!(signed_block.block_signature(), send.block_signature());
    assert_eq!(signed_block.hash(), send.hash());
}

#[test]
fn sign_without_key() {
    let mut system = System::new();
    let node = system.make_node();

    let server = setup_rpc_client_and_server(node.clone(), false);

    let send = Block::State(StateBlock::new(
        *DEV_GENESIS_ACCOUNT,
        *DEV_GENESIS_HASH,
        *DEV_GENESIS_PUB_KEY,
        Amount::MAX - Amount::raw(1_000_000),
        Account::from(PublicKey::zero()).into(),
        &DEV_GENESIS_KEY,
        node.work_generate_dev(*DEV_GENESIS_HASH),
    ));

    let result = node
        .runtime
        .block_on(async { server.client.sign(send.json_representation()).await });

    assert_eq!(
        result.err().map(|e| e.to_string()),
        Some(
            "node returned error: \"Private key or local wallet and account required\"".to_string()
        )
    );
}
