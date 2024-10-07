use rsnano_core::{Account, Amount, BlockHash};
use serde::{
    de::{self, MapAccess, Visitor},
    ser::SerializeMap,
    Deserialize, Deserializer, Serialize, Serializer,
};
use std::{collections::HashMap, fmt};

#[macro_export]
macro_rules! create_rpc_message {
    ($name:ident, $value_type:ty) => {
        #[derive(Debug, PartialEq, Eq)]
        pub struct $name {
            pub key: String,
            pub value: $value_type,
        }

        impl $name {
            pub fn new(key: String, value: $value_type) -> Self {
                Self { key, value }
            }
        }

        impl Serialize for $name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                let mut map = serializer.serialize_map(Some(1))?;
                map.serialize_entry(&self.key, &self.value)?;
                map.end()
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                struct DtoVisitor;

                impl<'de> Visitor<'de> for DtoVisitor {
                    type Value = $name;

                    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                        formatter.write_str("a map with a single key-value pair")
                    }

                    fn visit_map<A>(self, mut map: A) -> Result<$name, A::Error>
                    where
                        A: MapAccess<'de>,
                    {
                        let (key, value): (String, $value_type) = match map.next_entry()? {
                            Some(pair) => pair,
                            None => return Err(de::Error::invalid_length(0, &self)),
                        };

                        if map.next_entry::<String, $value_type>()?.is_some() {
                            return Err(de::Error::custom(
                                "Found more than one key-value pair in the map",
                            ));
                        }

                        Ok($name { key, value })
                    }
                }

                deserializer.deserialize_map(DtoVisitor)
            }
        }
    };
}

create_rpc_message!(BoolDto, bool);
create_rpc_message!(AccountRpcMessage, Account);
create_rpc_message!(AmountDto, Amount);
create_rpc_message!(BlockHashRpcMessage, BlockHash);
create_rpc_message!(BlocksHashesRpcMessage, Vec<BlockHash>);
create_rpc_message!(U64RpcMessage, u64);
create_rpc_message!(AccountsWithAmountsDto, HashMap<Account, Amount>);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AccountRpcMessage, AmountDto, BlockHashRpcMessage, BoolDto, U64RpcMessage};
    use rsnano_core::{Account, Amount, BlockHash};
    use serde_json::{from_str, to_string_pretty};
    use crate::AccountsWithAmountsDto;
    use std::collections::HashMap;

    #[test]
    fn serialize_bool_dto() {
        let bool_dto = BoolDto::new("key".to_string(), true);
        assert_eq!(
            serde_json::to_string_pretty(&bool_dto).unwrap(),
            r#"{
  "key": true
}"#
        );
    }

    #[test]
    fn deserialize_bool_dto() {
        let bool_dto = BoolDto::new("flag".to_string(), true);
        let serialized = to_string_pretty(&bool_dto).unwrap();
        let deserialized: BoolDto = from_str(&serialized).unwrap();
        assert_eq!(bool_dto, deserialized);
    }

    #[test]
    fn serialize_account_rpc_message() {
        assert_eq!(
            serde_json::to_string_pretty(&AccountRpcMessage::new(
                "account".to_string(),
                Account::from(123)
            ))
            .unwrap(),
            r#"{
  "account": "nano_111111111111111111111111111111111111111111111111115uwdgas549"
}"#
        )
    }

    #[test]
    fn derialize_account_rpc_message() {
        let account = Account::from(123);
        let account_arg = AccountRpcMessage::new("account".to_string(), account);
        let serialized = to_string_pretty(&account_arg).unwrap();
        let deserialized: AccountRpcMessage = from_str(&serialized).unwrap();
        assert_eq!(account_arg, deserialized)
    }

    #[test]
    fn serialize_amount_rpc_message() {
        let amount = Amount::raw(1000);
        let amount_rpc = AmountDto::new("amount".to_string(), amount);
        assert_eq!(
            serde_json::to_string_pretty(&amount_rpc).unwrap(),
            r#"{
  "amount": "1000"
}"#
        );
    }

    #[test]
    fn deserialize_amount_rpc_message() {
        let amount = Amount::from(1000);
        let amount_rpc = AmountDto::new("amount".to_string(), amount);
        let serialized = to_string_pretty(&amount_rpc).unwrap();
        let deserialized: AmountDto = from_str(&serialized).unwrap();
        assert_eq!(amount_rpc, deserialized);
    }

    #[test]
    fn serialize_block_hash_rpc_message() {
        let block_hash = BlockHash::from(123);
        let block_hash_message = BlockHashRpcMessage::new("block_hash".to_string(), block_hash);
        assert_eq!(
            serde_json::to_string_pretty(&block_hash_message).unwrap(),
            r#"{
  "block_hash": "000000000000000000000000000000000000000000000000000000000000007B"
}"#
        );
    }

    #[test]
    fn deserialize_block_hash_rpc_message() {
        let block_hash = BlockHash::from(123);
        let block_hash_message = BlockHashRpcMessage::new("block_hash".to_string(), block_hash);
        let serialized = to_string_pretty(&block_hash_message).unwrap();
        let deserialized: BlockHashRpcMessage = from_str(&serialized).unwrap();
        assert_eq!(block_hash_message, deserialized);
    }

    #[test]
    fn serialize_u64_rpc_message() {
        let block_hash_message = U64RpcMessage::new("key".to_string(), 1);
        assert_eq!(
            serde_json::to_string_pretty(&block_hash_message).unwrap(),
            r#"{
  "key": 1
}"#
        );
    }

    #[test]
    fn deserialize_u64_rpc_message() {
        let block_hash_message = U64RpcMessage::new("key".to_string(), 1);
        let serialized = to_string_pretty(&block_hash_message).unwrap();
        let deserialized: U64RpcMessage = from_str(&serialized).unwrap();
        assert_eq!(block_hash_message, deserialized);
    }

    #[test]
    fn serialize_accounts_with_amounts_dto() {
        let mut accounts = HashMap::new();
        accounts.insert(Account::zero(), Amount::from(1000));
        
        let message = AccountsWithAmountsDto::new("accounts".to_string(), accounts);
        
        let serialized = serde_json::to_string_pretty(&message).unwrap();
        assert_eq!(
            serialized,
            r#"{
  "accounts": {
    "nano_1111111111111111111111111111111111111111111111111111hifc8npp": "1000"
  }
}"#
        );
    }

    #[test]
    fn deserialize_accounts_with_amounts_dto() {
        let json = r#"{
  "accounts": {
    "nano_1111111111111111111111111111111111111111111111111111hifc8npp": "1000"
  }
}"#;

        let deserialized: AccountsWithAmountsDto = serde_json::from_str(json).unwrap();
        
        assert_eq!(deserialized.key, "accounts");
        assert_eq!(deserialized.value.get(&Account::zero()), Some(&Amount::from(1000)));
    }
}
