use rsnano_core::BlockHash;
use test_helpers::System;

async fn no_peers() {
    let mut system = System::new();
    let node = system.make_node();
    let hash = BlockHash::from(1);
    let work = node
        .runtime
        .block_on(node.distributed_work.cancel(root));
}
