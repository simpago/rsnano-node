use crate::{
    message_recorder::{make_node_callbacks, MessageRecorder},
    node_runner::{NodeRunner, NodeState},
};
use rsnano_node::Node;
use rsnano_nullable_clock::SteadyClock;
use std::sync::Arc;

pub(crate) struct NodeRunnerViewModel {
    msg_recorder: Arc<MessageRecorder>,
    clock: Arc<SteadyClock>,
    pub node_runner: NodeRunner,
}
impl NodeRunnerViewModel {
    pub(crate) fn new(
        node_runner: NodeRunner,
        msg_recorder: Arc<MessageRecorder>,
        clock: Arc<SteadyClock>,
    ) -> Self {
        Self {
            node_runner,
            msg_recorder,
            clock,
        }
    }

    pub(crate) fn can_start_node(&self) -> bool {
        self.node_runner.state() == NodeState::Stopped
    }

    pub(crate) fn can_stop_node(&self) -> bool {
        self.node_runner.state() == NodeState::Started
    }

    pub(crate) fn start_live_node(&mut self) {
        let callbacks = make_node_callbacks(self.msg_recorder.clone(), self.clock.clone());
        self.node_runner.start_live_node(callbacks);
    }

    pub(crate) fn start_beta_node(&mut self) {
        let callbacks = make_node_callbacks(self.msg_recorder.clone(), self.clock.clone());
        self.node_runner.start_beta_node(callbacks);
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

    pub(crate) fn node(&self) -> Option<&Node> {
        self.node_runner.node()
    }
}
