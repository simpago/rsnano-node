use crate::{
    message_recorder::{make_node_callbacks, MessageRecorder},
    node_runner::{NodeRunner, NodeState},
};
use rsnano_core::Networks;
use rsnano_node::{working_path_for, Node};
use rsnano_nullable_clock::SteadyClock;
use std::sync::Arc;

pub(crate) struct NodeRunnerViewModel {
    msg_recorder: Arc<MessageRecorder>,
    clock: Arc<SteadyClock>,
    pub node_runner: NodeRunner,
    network: Networks,
    pub data_path: String,
}
impl NodeRunnerViewModel {
    pub(crate) fn new(
        node_runner: NodeRunner,
        msg_recorder: Arc<MessageRecorder>,
        clock: Arc<SteadyClock>,
    ) -> Self {
        let network = Networks::NanoLiveNetwork;
        let mut model = Self {
            node_runner,
            msg_recorder,
            clock,
            network,
            data_path: String::new(),
        };
        model.set_network(Networks::NanoLiveNetwork);
        model
    }

    pub(crate) fn can_start_node(&self) -> bool {
        self.node_runner.state() == NodeState::Stopped
    }

    pub(crate) fn can_stop_node(&self) -> bool {
        self.node_runner.state() == NodeState::Started
    }

    pub(crate) fn start_node(&mut self) {
        let callbacks = make_node_callbacks(self.msg_recorder.clone(), self.clock.clone());
        self.node_runner
            .start_node(self.network, &self.data_path, callbacks);
    }

    pub(crate) fn stop_node(&mut self) {
        self.node_runner.stop();
    }

    pub(crate) fn status(&self) -> &'static str {
        match self.node_runner.state() {
            NodeState::Starting => "starting...",
            NodeState::Started => "running",
            NodeState::Stopping => "stopping...",
            NodeState::Stopped => "not running",
        }
    }

    pub(crate) fn node(&self) -> Option<Arc<Node>> {
        self.node_runner.node()
    }

    pub(crate) fn network(&self) -> Networks {
        self.network
    }

    pub(crate) fn set_network(&mut self, network: Networks) {
        self.network = network;
        self.data_path = working_path_for(network)
            .unwrap()
            .to_str()
            .unwrap()
            .to_owned();
    }
}
