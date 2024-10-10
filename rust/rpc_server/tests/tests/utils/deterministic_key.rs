use rsnano_core::{Account, PublicKey, RawKey};
use test_helpers::{setup_rpc_client_and_server, System};

#[test]
fn deterministic_key() {
    let mut system = System::new();
    let node = system.make_node();

    let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), false);

    let result = node.runtime.block_on(async {
        rpc_client
            .deterministic_key(RawKey::zero(), 0)
            .await
            .unwrap()
    });

    assert_eq!(
        result.private,
        RawKey::decode_hex("9F0E444C69F77A49BD0BE89DB92C38FE713E0963165CCA12FAF5712D7657120F")
            .unwrap()
    );

    assert_eq!(
        result.public,
        PublicKey::decode_hex("C008B814A7D269A1FA3C6528B19201A24D797912DB9996FF02A1FF356E45552B")
            .unwrap()
    );

    assert_eq!(
        result.account,
        Account::decode_account(
            "nano_3i1aq1cchnmbn9x5rsbap8b15akfh7wj7pwskuzi7ahz8oq6cobd99d4r3b7"
        )
        .unwrap()
    );

    server.abort();
}
