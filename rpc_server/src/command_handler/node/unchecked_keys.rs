use crate::command_handler::RpcCommandHandler;
use rsnano_rpc_messages::{UncheckedKeyDto, UncheckedKeysArgs, UncheckedKeysResponse};
use std::cell::RefCell;

impl RpcCommandHandler {
    pub(crate) fn unchecked_keys(&self, args: UncheckedKeysArgs) -> UncheckedKeysResponse {
        let count = args.count.unwrap_or(u64::MAX.into()).inner();
        let unchecked_keys = RefCell::new(Vec::new());

        self.node.unchecked.for_each_with_dependency(
            &args.key,
            |key, info| {
                let key_dto = UncheckedKeyDto {
                    key: key.previous,
                    hash: info.block.hash(),
                    modified_timestamp: info.modified.into(),
                    contents: info.block.json_representation(),
                };
                unchecked_keys.borrow_mut().push(key_dto);
            },
            || (unchecked_keys.borrow().len() as u64) < count,
        );

        UncheckedKeysResponse::new(unchecked_keys.take())
    }
}
