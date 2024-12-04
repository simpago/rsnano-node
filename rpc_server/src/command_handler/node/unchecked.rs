use crate::command_handler::RpcCommandHandler;
use rsnano_core::{UncheckedInfo, UncheckedKey};
use rsnano_rpc_messages::{CountArgs, UncheckedResponse};
use std::collections::HashMap;

impl RpcCommandHandler {
    pub(crate) fn unchecked(&self, args: CountArgs) -> UncheckedResponse {
        let count = args.count.map(|i| u64::from(i)).unwrap_or(u64::MAX);
        let mut blocks = HashMap::new();

        let mut iterations = 0;
        self.node.unchecked.for_each(
            |_key: &UncheckedKey, info: &UncheckedInfo| {
                let json_block = info.block.json_representation();
                blocks.insert(info.block.hash(), json_block);
            },
            || {
                iterations += 1;
                iterations <= count
            },
        );

        UncheckedResponse::new(blocks)
    }
}
