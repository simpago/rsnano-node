use rsnano_core::{HashOrAccount, UncheckedInfo, UncheckedKey};
use rsnano_node::Node;
use rsnano_rpc_messages::{UncheckedKeyDto, UncheckedKeysDto};
use serde_json::to_string_pretty;
use std::sync::{Arc, Mutex};

pub async fn unchecked_keys(node: Arc<Node>, key: HashOrAccount, count: u64) -> String {
    let unchecked_keys = Arc::new(Mutex::new(Vec::new()));

    node.unchecked.for_each_with_dependency(
        &key,
        &mut {
            let unchecked_keys = Arc::clone(&unchecked_keys);
            move |unchecked_key: &UncheckedKey, info: &UncheckedInfo| {
                let mut unchecked_keys_guard = unchecked_keys.lock().unwrap();
                if unchecked_keys_guard.len() < count as usize {
                    if let Some(block) = info.block.as_ref() {
                        let dto = UncheckedKeyDto::new(
                            unchecked_key.hash,
                            block.hash(),
                            info.modified,
                            block.json_representation(),
                        );
                        unchecked_keys_guard.push(dto);
                    }
                }
            }
        },
        &Box::new(|| unchecked_keys.lock().unwrap().len() < count as usize),
    );

    let unchecked_keys = Arc::try_unwrap(unchecked_keys)
        .unwrap()
        .into_inner()
        .unwrap();
    to_string_pretty(&UncheckedKeysDto::new(unchecked_keys)).unwrap()
}
