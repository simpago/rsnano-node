use rsnano_core::{Account, PublicKey, RawKey};
use rsnano_rpc_messages::KeyPairDto;
use serde_json::to_string_pretty;

pub async fn key_expand(key: RawKey) -> String {
    let public: PublicKey = (&key).try_into().unwrap();
    let account = Account::from(public);

    to_string_pretty(&KeyPairDto::new(key, public, account)).unwrap()
}

#[cfg(test)]
mod tests {
    use rsnano_core::{Account, PublicKey, RawKey};
    use test_helpers::{setup_rpc_client_and_server, System};

    #[test]
    fn key_expand() {
        let mut system = System::new();
        let node = system.make_node();

        let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), false);

        let result = node.tokio.block_on(async {
            rpc_client
                .key_expand(
                    RawKey::decode_hex(
                        "781186FB9EF17DB6E3D1056550D9FAE5D5BBADA6A6BC370E4CBB938B1DC71DA3",
                    )
                    .unwrap(),
                )
                .await
                .unwrap()
        });

        assert_eq!(
            result.private,
            RawKey::decode_hex("781186FB9EF17DB6E3D1056550D9FAE5D5BBADA6A6BC370E4CBB938B1DC71DA3")
                .unwrap()
        );

        assert_eq!(
            result.public,
            PublicKey::decode_hex(
                "3068BB1CA04525BB0E416C485FE6A67FD52540227D267CC8B6E8DA958A7FA039"
            )
            .unwrap()
        );

        assert_eq!(
            result.account,
            Account::decode_account(
                "nano_1e5aqegc1jb7qe964u4adzmcezyo6o146zb8hm6dft8tkp79za3sxwjym5rx"
            )
            .unwrap()
        );

        server.abort();
    }
}
