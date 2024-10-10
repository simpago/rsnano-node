use rsnano_core::{UncheckedInfo, UncheckedKey};
use rsnano_node::Node;
use rsnano_rpc_messages::UncheckedDto;
use serde_json::to_string_pretty;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub async fn unchecked(node: Arc<Node>, count: u64) -> String {
    let blocks = Arc::new(Mutex::new(HashMap::new()));

    node.unchecked.for_each(
        {
            let blocks = Arc::clone(&blocks);
            Box::new(move |_key: &UncheckedKey, info: &UncheckedInfo| {
                let mut blocks_guard = blocks.lock().unwrap();
                if blocks_guard.len() < count as usize {
                    if let Some(block) = info.block.as_ref() {
                        let hash = block.hash();
                        let json_block = block.json_representation();
                        blocks_guard.insert(hash, json_block);
                    }
                }
            })
        },
        Box::new(|| true),
    );

    let blocks = Arc::try_unwrap(blocks).unwrap().into_inner().unwrap();
    to_string_pretty(&UncheckedDto::new(blocks)).unwrap()
}
