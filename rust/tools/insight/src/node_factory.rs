use rsnano_core::Networks;
use rsnano_node::{Node, NodeBuilder, NodeCallbacks};
use std::sync::Arc;

pub(crate) struct NodeFactory {
    runtime: tokio::runtime::Handle,
    is_nulled: bool,
}

impl NodeFactory {
    pub(crate) fn new(runtime: tokio::runtime::Handle) -> Self {
        Self {
            runtime,
            is_nulled: false,
        }
    }

    #[allow(dead_code)]
    pub(crate) fn new_null() -> Self {
        Self {
            runtime: tokio::runtime::Handle::current(),
            is_nulled: true,
        }
    }

    pub(crate) fn create_node(&self, network: Networks, callbacks: NodeCallbacks) -> Arc<Node> {
        if self.is_nulled {
            Arc::new(Node::new_null_with_callbacks(callbacks))
        } else {
            NodeBuilder::new(network)
                .runtime(self.runtime.clone())
                .callbacks(callbacks)
                .finish()
                .unwrap()
                .into()
        }
    }
}

impl Default for NodeFactory {
    fn default() -> Self {
        Self::new(tokio::runtime::Handle::current())
    }
}
