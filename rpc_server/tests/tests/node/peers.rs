use rsnano_rpc_messages::PeersDto;
use test_helpers::{setup_rpc_client_and_server, System};

#[test]
fn peers_without_details() {
    let mut system = System::new();
    let node1 = system.make_node();
    let _node2 = system.make_node();

    let server = setup_rpc_client_and_server(node1.clone(), false);

    let result = node1
        .runtime
        .block_on(async { server.client.peers(None).await })
        .unwrap();

    match result {
        PeersDto::Simple(peers) => {
            assert!(!peers.peers.is_empty());
        }
        PeersDto::Detailed(_) => panic!("Expected Simple peer data"),
    }
}

#[test]
fn peers_with_details() {
    let mut system = System::new();
    let node1 = system.make_node();
    let _node2 = system.make_node();

    let server = setup_rpc_client_and_server(node1.clone(), false);

    let result = node1
        .runtime
        .block_on(async { server.client.peers(Some(true)).await.unwrap() });

    println!("{:?}", result);

    match result {
        PeersDto::Detailed(peers) => {
            assert!(!peers.peers.is_empty());
        }
        PeersDto::Simple(_) => panic!("Expected Detailed peer data"),
    }
}
