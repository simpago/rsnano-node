use rsnano_node::{node::Node, wallets::WalletsExt};
use rsnano_rpc_messages::{ErrorDto, WalletChangeSeedArgs, WalletChangeSeedDto};
use serde_json::to_string_pretty;
use std::sync::Arc;

pub async fn wallet_change_seed(
    node: Arc<Node>,
    enable_control: bool,
    args: WalletChangeSeedArgs,
) -> String {
    if enable_control {
        let (restored_count, last_restored_account) = node
            .wallets
            .change_seed(args.wallet, &args.seed, args.count.unwrap_or(0))
            .unwrap();
        to_string_pretty(&WalletChangeSeedDto::new(
            last_restored_account,
            restored_count,
        ))
        .unwrap()
    } else {
        to_string_pretty(&ErrorDto::new("RPC control is disabled".to_string())).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use rsnano_core::{RawKey, WalletId};
    use rsnano_node::wallets::WalletsExt;
    use test_helpers::{setup_rpc_client_and_server, System};

    #[test]
    fn wallet_change_seed() {
        let mut system = System::new();
        let node = system.make_node();

        let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), true);

        let wallet_id = WalletId::zero();
        node.wallets.create(wallet_id);
        let new_seed =
            RawKey::decode_hex("74F2B37AAD20F4A260F0A5B3CB3D7FB51673212263E58A380BC10474BB039CEE")
                .unwrap();

        node.tokio.block_on(async {
            rpc_client
                .wallet_change_seed(wallet_id, new_seed, None)
                .await
                .unwrap()
        });

        assert_eq!(node.wallets.get_seed(wallet_id).unwrap(), new_seed);

        server.abort();
    }

    #[test]
    fn wallet_change_seed_fails_without_enable_control() {
        let mut system = System::new();
        let node = system.make_node();

        let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), false);

        let result = node.tokio.block_on(async {
            rpc_client
                .wallet_change_seed(WalletId::zero(), RawKey::zero(), None)
                .await
        });

        assert_eq!(
            result.err().map(|e| e.to_string()),
            Some("node returned error: \"RPC control is disabled\"".to_string())
        );

        server.abort();
    }
}
