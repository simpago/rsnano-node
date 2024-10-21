use crate::{node_factory::NodeFactory, nullable_runtime::NullableRuntime};
use num::FromPrimitive;
use num_derive::FromPrimitive;
use rsnano_core::Networks;
use rsnano_node::{Node, NodeCallbacks, NodeExt};
use std::sync::{
    atomic::{AtomicU8, Ordering},
    Arc,
};

#[derive(FromPrimitive, PartialEq, Eq)]
pub enum NodeState {
    Starting,
    Started,
    Stopping,
    Stopped,
}

pub(crate) struct NodeRunner {
    node_factory: NodeFactory,
    runtime: Arc<NullableRuntime>,
    node: Option<Arc<Node>>,
    state: Arc<AtomicU8>,
}

impl NodeRunner {
    pub(crate) fn new(runtime: Arc<NullableRuntime>, node_factory: NodeFactory) -> Self {
        Self {
            node_factory,
            runtime,
            node: None,
            state: Arc::new(AtomicU8::new(NodeState::Stopped as u8)),
        }
    }

    pub(crate) fn start_live_node(&mut self, callbacks: NodeCallbacks) {
        self.start_node(Networks::NanoLiveNetwork, callbacks);
    }

    pub(crate) fn start_beta_node(&mut self, callbacks: NodeCallbacks) {
        self.start_node(Networks::NanoBetaNetwork, callbacks);
    }

    pub fn start_node(&mut self, network: Networks, callbacks: NodeCallbacks) {
        let node = self.node_factory.create_node(network, callbacks);

        let node2 = node.clone();

        self.state
            .store(NodeState::Starting as u8, Ordering::SeqCst);
        self.node = Some(node);

        let state = self.state.clone();
        self.runtime.spawn_blocking(move || {
            node2.start();
            state.store(NodeState::Started as u8, Ordering::SeqCst);
        });
    }

    pub(crate) fn stop(&mut self) {
        if let Some(node) = self.node.take() {
            {
                self.state
                    .store(NodeState::Stopping as u8, Ordering::SeqCst);
                let state = self.state.clone();
                self.runtime.spawn_blocking(move || {
                    node.stop();
                    state.store(NodeState::Stopped as u8, Ordering::SeqCst);
                });
            }
        }
    }

    pub(crate) fn state(&self) -> NodeState {
        FromPrimitive::from_u8(self.state.load(Ordering::SeqCst)).unwrap()
    }

    pub(crate) fn node(&self) -> Option<&Node> {
        self.node.as_ref().map(|n| &**n)
    }
}

impl Drop for NodeRunner {
    fn drop(&mut self) {
        if let Some(node) = self.node.take() {
            node.stop();
        }
    }
}
