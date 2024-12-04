use crate::{BlockSubTypeDto, RpcBool, RpcCommand};
use rsnano_core::JsonBlock;
use serde::{Deserialize, Serialize};

impl RpcCommand {
    pub fn process(process_args: ProcessArgs) -> Self {
        Self::Process(process_args)
    }
}

impl From<JsonBlock> for ProcessArgs {
    fn from(value: JsonBlock) -> Self {
        Self::build(value).finish()
    }
}

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct ProcessArgs {
    pub json_block: RpcBool,
    pub block: JsonBlock,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subtype: Option<BlockSubTypeDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub force: Option<RpcBool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub watch_work: Option<RpcBool>,
    #[serde(rename = "async", skip_serializing_if = "Option::is_none")]
    pub is_async: Option<RpcBool>,
}

impl ProcessArgs {
    pub fn build(block: JsonBlock) -> ProcessArgsBuilder {
        ProcessArgsBuilder {
            args: ProcessArgs {
                json_block: true.into(),
                subtype: None,
                block,
                force: None,
                watch_work: None,
                is_async: None,
            },
        }
    }
}

pub struct ProcessArgsBuilder {
    args: ProcessArgs,
}

impl ProcessArgsBuilder {
    pub fn subtype(mut self, subtype: BlockSubTypeDto) -> Self {
        self.args.subtype = Some(subtype);
        self
    }

    pub fn force(mut self) -> Self {
        self.args.force = Some(true.into());
        self
    }

    pub fn as_async(mut self) -> Self {
        self.args.is_async = Some(true.into());
        self
    }

    pub fn without_watch_work(mut self) -> Self {
        self.args.watch_work = Some(false.into());
        self
    }

    pub fn finish(self) -> ProcessArgs {
        self.args
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rsnano_core::Block;
    use serde_json::json;

    #[test]
    fn test_process_command_serialize() {
        let process_args = ProcessArgs::build(Block::new_test_instance().json_representation())
            .subtype(BlockSubTypeDto::Send)
            .force()
            .as_async()
            .without_watch_work()
            .finish();
        let command = RpcCommand::Process(process_args);

        let serialized = serde_json::to_value(&command).unwrap();
        assert_eq!(
            serialized,
            json!({
                "action": "process",
                "json_block": "true",
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
                "force": "true",
                "watch_work": "false",
                "async": "true"
            })
        );
    }

    #[test]
    fn test_process_command_deserialize() {
        let json = json!({
            "action": "process",
            "subtype": "receive",
            "json_block": "true",
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
            "force": "false",
            "watch_work": "true",
            "async": "false"
        });

        let deserialized: RpcCommand = serde_json::from_value(json).unwrap();
        if let RpcCommand::Process(args) = deserialized {
            assert_eq!(args.subtype, Some(BlockSubTypeDto::Receive));
            assert_eq!(args.block, Block::new_test_instance().json_representation());
            assert_eq!(args.force, Some(false.into()));
            assert_eq!(args.watch_work, Some(true.into()));
            assert_eq!(args.is_async, Some(false.into()));
        } else {
            panic!("Deserialized to wrong variant");
        }
    }
}
