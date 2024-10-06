use rsnano_node::{bootstrap::BootstrapInitiatorExt, node::Node};
use rsnano_rpc_messages::SuccessDto;
use serde_json::to_string_pretty;
use std::{
    net::{Ipv6Addr, SocketAddrV6},
    sync::Arc,
};

pub async fn bootstrap(
    node: Arc<Node>,
    address: Ipv6Addr,
    port: u16,
    id: Option<String>,
) -> String {
    let id = id.unwrap_or(String::new());
    let endpoint = SocketAddrV6::new(address, port, 0, 0);
    node.bootstrap_initiator.bootstrap2(endpoint, id);

    to_string_pretty(&SuccessDto::new()).unwrap()
}

#[cfg(test)]
mod tests {
    use crate::service::responses::test_helpers::setup_rpc_client_and_server;
    use rsnano_core::{
        Amount, BlockEnum, BlockHash, KeyPair, StateBlock, WalletId, DEV_GENESIS_KEY,
    };
    use rsnano_ledger::{DEV_GENESIS_ACCOUNT, DEV_GENESIS_HASH, DEV_GENESIS_PUB_KEY};
    use rsnano_network::ChannelMode;
    use rsnano_node::{
        bootstrap::BootstrapInitiatorExt, config::{FrontiersConfirmationMode, NodeConfig, NodeFlags}, wallets::WalletsExt
    };
    use std::{net::{Ipv6Addr, SocketAddrV6}, time::Duration};
    use test_helpers::{assert_timely, assert_timely_eq, establish_tcp, System};

    #[test]
    fn bootstrap_id_none() {
        let mut system = System::new();
        let key = KeyPair::new();
        let node1 = system.make_disconnected_node();
        let node_clone = node1.clone();
        let node_clone2 = node1.clone();
        let (rpc_client, server) = setup_rpc_client_and_server(node1.clone(), true);
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
        let send1 = BlockEnum::State(StateBlock::new(
            *DEV_GENESIS_ACCOUNT,
            *DEV_GENESIS_HASH,
            *DEV_GENESIS_PUB_KEY,
            Amount::zero(),
            key.account().into(),
            &DEV_GENESIS_KEY,
            node1.work_generate_dev((*DEV_GENESIS_HASH).into()),
        ));
        node1.process(send1.clone()).unwrap();

        // open key account receiving all balance of genesis
        let open = BlockEnum::State(StateBlock::new(
            key.account(),
            BlockHash::zero(),
            key.public_key(),
            Amount::MAX,
            send1.hash().into(),
            &key,
            node1.work_generate_dev(key.public_key().into()),
        ));
        node1.process(open.clone()).unwrap();

        // send from key to genesis 100 raw
        let send2 = BlockEnum::State(StateBlock::new(
            key.account(),
            open.hash(),
            key.public_key(),
            Amount::MAX - Amount::raw(100),
            (*DEV_GENESIS_ACCOUNT).into(),
            &key,
            node1.work_generate_dev(open.hash().into()),
        ));
        node1.process(send2.clone()).unwrap();

        // receive the 100 raw on genesis
        let receive = BlockEnum::State(StateBlock::new(
            *DEV_GENESIS_ACCOUNT,
            send1.hash(),
            *DEV_GENESIS_PUB_KEY,
            Amount::raw(100),
            send2.hash().into(),
            &DEV_GENESIS_KEY,
            node1.work_generate_dev(send1.hash().into()),
        ));
        node1.process(receive.clone()).unwrap();

        let config = NodeConfig {
            frontiers_confirmation: FrontiersConfirmationMode::Disabled,
            ..System::default_config()
        };

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

        let endpoint = SocketAddrV6::new(address, port, 0, 0);
        //node1.bootstrap_initiator.bootstrap2(endpoint, String::new());

        let node2_clone = node2.clone();

        let result = node1.tokio.spawn(async move {
            rpc_client
                .bootstrap(
                    address,
                    port,
                    None,
                )
                .await
                .unwrap();
        });

        //assert_timely(
            //std::time::Duration::from_secs(10),
            //|| node_clone2.tokio.block_on(async { result.is_finished() })
        //);

        /*assert_timely_eq(
            Duration::from_secs(5),
            || node2.balance(&DEV_GENESIS_ACCOUNT),
            Amount::raw(100),
        );*/

        server.abort();
    }
}