use rsnano_rpc_messages::PeerData;
use test_helpers::{setup_rpc_client_and_server, System};

#[test]
fn peers_without_details() {
    let mut system = System::new();
    let node1 = system.make_node();
    let _node2 = system.make_node();

    let (rpc_client, server) = setup_rpc_client_and_server(node1.clone(), false);

    let result = node1
        .runtime
        .block_on(async { rpc_client.peers(None).await.unwrap() });

    match result.peers {
        PeerData::Simple(peers) => {
            assert!(!peers.is_empty());
        }
        PeerData::Detailed(_) => panic!("Expected Simple peer data"),
    }

    server.abort();
}

#[test]
fn peers_with_details() {
    let mut system = System::new();
    let node1 = system.make_node();
    let _node2 = system.make_node();

    let (rpc_client, server) = setup_rpc_client_and_server(node1.clone(), false);

    let result = node1
        .runtime
        .block_on(async { rpc_client.peers(Some(true)).await.unwrap() });

    println!("{:?}", result);

    match result.peers {
        PeerData::Detailed(peers) => {
            assert!(!peers.is_empty());
        }
        PeerData::Simple(_) => panic!("Expected Detailed peer data"),
    }

    server.abort();
}
