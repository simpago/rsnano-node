use rsnano_core::{Amount, Block, BlockHash, PrivateKey, StateBlock, WalletId, DEV_GENESIS_KEY};
use rsnano_ledger::{DEV_GENESIS_ACCOUNT, DEV_GENESIS_HASH, DEV_GENESIS_PUB_KEY};
use rsnano_network::ChannelMode;
use rsnano_node::{config::NodeFlags, wallets::WalletsExt};
use rsnano_rpc_messages::BootstrapArgs;
use std::time::Duration;
use test_helpers::{assert_timely_eq, setup_rpc_client_and_server, System};

#[test]
fn bootstrap_id_none() {
    let mut system = System::new();
    let key = PrivateKey::new();
    let node1 = system.make_disconnected_node();
    let server = setup_rpc_client_and_server(node1.clone(), true);

    let wallet_id = WalletId::from(100);
    node1.wallets.create(wallet_id);
    node1
        .wallets
        .insert_adhoc2(&wallet_id, &DEV_GENESIS_KEY.private_key(), true)
        .unwrap();
    node1
        .wallets
        .insert_adhoc2(&wallet_id, &key.private_key(), true)
        .unwrap();

    // send all balance from genesis to key
    let send1 = Block::State(StateBlock::new(
        *DEV_GENESIS_ACCOUNT,
        *DEV_GENESIS_HASH,
        *DEV_GENESIS_PUB_KEY,
        Amount::zero(),
        key.account().into(),
        &DEV_GENESIS_KEY,
        node1.work_generate_dev(*DEV_GENESIS_HASH),
    ));
    node1.process(send1.clone()).unwrap();

    // open key account receiving all balance of genesis
    let open = Block::State(StateBlock::new(
        key.account(),
        BlockHash::zero(),
        key.public_key(),
        Amount::MAX,
        send1.hash().into(),
        &key,
        node1.work_generate_dev(key.public_key()),
    ));
    node1.process(open.clone()).unwrap();

    // send from key to genesis 100 raw
    let send2 = Block::State(StateBlock::new(
        key.account(),
        open.hash(),
        key.public_key(),
        Amount::MAX - Amount::raw(100),
        (*DEV_GENESIS_ACCOUNT).into(),
        &key,
        node1.work_generate_dev(open.hash()),
    ));
    node1.process(send2.clone()).unwrap();

    // receive the 100 raw on genesis
    let receive = Block::State(StateBlock::new(
        *DEV_GENESIS_ACCOUNT,
        send1.hash(),
        *DEV_GENESIS_PUB_KEY,
        Amount::raw(100),
        send2.hash().into(),
        &DEV_GENESIS_KEY,
        node1.work_generate_dev(send1.hash()),
    ));
    node1.process(receive.clone()).unwrap();

    let config = System::default_config_without_backlog_population();

    let flags = NodeFlags {
        disable_ongoing_bootstrap: true,
        disable_ascending_bootstrap: true,
        ..Default::default()
    };

    let node2 = system.build_node().config(config).flags(flags).finish();

    node1
        .peer_connector
        .connect_to(node2.tcp_listener.local_address());
    assert_timely_eq(
        Duration::from_secs(5),
        || {
            node2
                .network_info
                .read()
                .unwrap()
                .count_by_mode(ChannelMode::Realtime)
        },
        1,
    );

    let address = *node2.tcp_listener.local_address().ip();
    let port = node2.tcp_listener.local_address().port();

    node1.runtime.spawn(async move {
        server
            .client
            .bootstrap(BootstrapArgs::new(address, port))
            .await
            .unwrap();
    });

    // TODO: this fails because bootstrap2 also call block_on
    //assert_timely(
    //std::time::Duration::from_secs(10),
    //|| node1.tokio.block_on(async { result.is_finished() })
    //);

    /*assert_timely_eq(
        Duration::from_secs(5),
        || node2.balance(&DEV_GENESIS_ACCOUNT),
        Amount::raw(100),
    );*/
}
