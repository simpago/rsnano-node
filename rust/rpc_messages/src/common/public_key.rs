use rsnano_core::PublicKey;
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyRpcMessage {
    pub key: PublicKey,
}

impl KeyRpcMessage {
    pub fn new(key: PublicKey) -> Self {
        Self { key }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{self, from_str};

    #[test]
    fn serialize_key_rpc_message() {
        let dto = KeyRpcMessage {
            key: PublicKey::zero(),
        };

        let serialized = serde_json::to_string(&dto).unwrap();

        let expected =
            r#"{"key":"0000000000000000000000000000000000000000000000000000000000000000"}"#;

        assert_eq!(serialized, expected);
    }

    #[test]
    fn deserialize_key_rpc_message() {
        let json_str =
            r#"{"key": "0000000000000000000000000000000000000000000000000000000000000000"}"#;
        let deserialized: KeyRpcMessage = from_str(json_str).unwrap();
        assert_eq!(
            deserialized,
            KeyRpcMessage {
                key: PublicKey::zero(),
            }
        );
    }
}
