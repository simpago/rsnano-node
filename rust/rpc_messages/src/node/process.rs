use crate::RpcCommand;
use rsnano_core::{BlockSubType, JsonBlock};
use serde::{Deserialize, Serialize};

impl RpcCommand {
    pub fn process(process_args: ProcessArgs) -> Self {
        Self::Process(process_args)
    }
}

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct ProcessArgs {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subtype: Option<BlockSubType>,
    pub block: JsonBlock,
    pub force: Option<bool>,
    pub watch_work: Option<bool>,
    #[serde(rename = "async")]
    pub is_async: Option<bool>,
}

impl ProcessArgs {
    pub fn new(
        subtype: Option<BlockSubType>,
        block: JsonBlock,
        force: Option<bool>,
        watch_work: Option<bool>,
        is_async: Option<bool>,
    ) -> Self {
        Self {
            subtype,
            block,
            force,
            watch_work,
            is_async,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rsnano_core::BlockEnum;
    use serde_json::json;

    #[test]
    fn test_process_command_serialize() {
        let process_args = ProcessArgs::new(
            Some(BlockSubType::Send),
            BlockEnum::new_test_instance().json_representation(),
            Some(true),
            Some(false),
            Some(true),
        );
        let command = RpcCommand::Process(process_args);

        let serialized = serde_json::to_value(&command).unwrap();
        assert_eq!(
            serialized,
            json!({
                "action": "process",
                "subtype": "send",
                "block": {
                    "type": "state",
                    "account": "nano_39y535msmkzb31bx73tdnf8iken5ucw9jt98re7nriduus6cgs6uonjdm8r5",
                    "previous": "00000000000000000000000000000000000000000000000000000000000001C8",
                    "representative": "nano_11111111111111111111111111111111111111111111111111ros3kc7wyy",
                    "balance": "420",
                    "link": "000000000000000000000000000000000000000000000000000000000000006F",
                    "link_as_account": "nano_111111111111111111111111111111111111111111111111115hkrzwewgm",
                    "signature": "F26EC6180795C63CFEC46F929DCF6269445208B6C1C837FA64925F1D61C218D4D263F9A73A4B76E3174888C6B842FC1380AC15183FA67E92B2091FEBCCBDB308",
                    "work": "0000000000010F2C"
                },
                "force": true,
                "watch_work": false,
                "async": true
            })
        );
    }

    #[test]
    fn test_process_command_deserialize() {
        let json = json!({
            "action": "process",
            "subtype": "receive",
            "block": {
                "type": "state",
                "account": "nano_39y535msmkzb31bx73tdnf8iken5ucw9jt98re7nriduus6cgs6uonjdm8r5",
                "previous": "00000000000000000000000000000000000000000000000000000000000001C8",
                "representative": "nano_11111111111111111111111111111111111111111111111111ros3kc7wyy",
                "balance": "420",
                "link": "000000000000000000000000000000000000000000000000000000000000006F",
                "link_as_account": "nano_111111111111111111111111111111111111111111111111115hkrzwewgm",
                "signature": "F26EC6180795C63CFEC46F929DCF6269445208B6C1C837FA64925F1D61C218D4D263F9A73A4B76E3174888C6B842FC1380AC15183FA67E92B2091FEBCCBDB308",
                "work": "0000000000010F2C"
            },
            "force": false,
            "watch_work": true,
            "async": false
        });

        let deserialized: RpcCommand = serde_json::from_value(json).unwrap();
        if let RpcCommand::Process(args) = deserialized {
            assert_eq!(args.subtype, Some(BlockSubType::Receive));
            assert_eq!(
                args.block,
                BlockEnum::new_test_instance().json_representation()
            );
            assert_eq!(args.force, Some(false));
            assert_eq!(args.watch_work, Some(true));
            assert_eq!(args.is_async, Some(false));
        } else {
            panic!("Deserialized to wrong variant");
        }
    }
}
