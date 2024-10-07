use rsnano_messages::{Keepalive, Message};
use rsnano_network::{DropPolicy, TrafficType};
use rsnano_node::node::Node;
use rsnano_rpc_messages::{ErrorDto, SuccessDto};
use serde_json::to_string_pretty;
use std::{
    net::{Ipv6Addr, SocketAddrV6},
    sync::Arc,
};

pub async fn keepalive(
    node: Arc<Node>,
    enable_control: bool,
    address: Ipv6Addr,
    port: u16,
) -> String {
    if !enable_control {
        return to_string_pretty(&ErrorDto::new("RPC control is disabled".to_string())).unwrap();
    }

    let peering_addr = SocketAddrV6::new(address.into(), port, 0, 0);
    let channel_id = node
        .network_info
        .read()
        .unwrap()
        .find_realtime_channel_by_peering_addr(&peering_addr);

    match channel_id {
        Some(id) => {
            let keepalive = Message::Keepalive(Keepalive::default());
            let mut publisher = node.message_publisher.lock().unwrap();

            publisher.try_send(
                id,
                &keepalive,
                DropPolicy::ShouldNotDrop,
                TrafficType::Generic,
            );

            to_string_pretty(&SuccessDto::new()).unwrap()
        }
        None => to_string_pretty(&ErrorDto::new("Peer not found".to_string())).unwrap(),
    }
}

#[cfg(test)]
mod tests {
    use rsnano_network::ChannelMode;
    use rsnano_node::stats::{DetailType, Direction, StatType};
    use std::{net::Ipv6Addr, time::Duration};
    use test_helpers::{
        assert_timely_eq, assert_timely_msg, establish_tcp, get_available_port,
        setup_rpc_client_and_server, System,
    };

    #[test]
    fn keepalive() {
        let mut system = System::new();
        let node0 = system.make_node();

        let (rpc_client, server) = setup_rpc_client_and_server(node0.clone(), true);

        let mut node1_config = System::default_config();
        node1_config.tcp_incoming_connections_max = 0; // Prevent ephemeral node1->node0 channel replacement with incoming connection
        let node1 = system
            .build_node()
            .config(node1_config)
            .disconnected()
            .finish();

        let channel1 = establish_tcp(&node1, &node0);
        assert_timely_eq(
            Duration::from_secs(3),
            || {
                node0
                    .network_info
                    .read()
                    .unwrap()
                    .count_by_mode(ChannelMode::Realtime)
            },
            1,
        );

        let channel0 = node0
            .network_info
            .read()
            .unwrap()
            .find_node_id(&node1.node_id.public_key())
            .unwrap()
            .clone();

        assert_eq!(channel0.local_addr(), channel1.peer_addr());
        assert_eq!(channel1.local_addr(), channel0.peer_addr());

        let timestamp_before_keepalive = channel0.last_activity();
        let keepalive_count =
            node0
                .stats
                .count(StatType::Message, DetailType::Keepalive, Direction::Out);

        assert_timely_msg(
            Duration::from_secs(3),
            || node0.steady_clock.now() > timestamp_before_keepalive,
            "clock did not advance",
        );

        node0.tokio.block_on(async {
            rpc_client
                .keepalive(Ipv6Addr::LOCALHOST, node1.config.peering_port.unwrap())
                .await
                .unwrap();
        });

        assert_timely_msg(
            Duration::from_secs(3),
            || {
                node0
                    .stats
                    .count(StatType::Message, DetailType::Keepalive, Direction::Out)
                    == keepalive_count + 1
            },
            "keepalive count",
        );

        assert_eq!(
            node0
                .network_info
                .read()
                .unwrap()
                .count_by_mode(ChannelMode::Realtime),
            1
        );

        let timestamp_after_keepalive = channel0.last_activity();
        assert!(timestamp_after_keepalive > timestamp_before_keepalive);

        server.abort();
    }

    #[test]
    fn keepalive_fails_without_rpc_control_enabled() {
        let mut system = System::new();
        let node = system.make_node();

        let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), false);

        let result = node.tokio.block_on(async {
            rpc_client
                .keepalive(Ipv6Addr::LOCALHOST, get_available_port())
                .await
        });

        assert_eq!(
            result.err().map(|e| e.to_string()),
            Some("node returned error: \"RPC control is disabled\"".to_string())
        );

        server.abort();
    }

    #[test]
    fn keepalive_fails_with_peer_not_found() {
        let mut system = System::new();
        let node = system.make_node();

        let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), true);

        let result = node.tokio.block_on(async {
            rpc_client
                .keepalive(Ipv6Addr::LOCALHOST, get_available_port())
                .await
        });

        assert_eq!(
            result.err().map(|e| e.to_string()),
            Some("node returned error: \"Peer not found\"".to_string())
        );

        server.abort();
    }
}
