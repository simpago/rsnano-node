use crate::{node_factory::NodeFactory, nullable_runtime::NullableRuntime};
use num_format::{Locale, ToFormattedString};
use rsnano_core::Networks;
use rsnano_messages::Message;
use rsnano_network::{ChannelDirection, ChannelId};
use rsnano_node::{Node, NodeCallbacks, NodeExt};
use std::sync::{
    atomic::{AtomicBool, AtomicUsize, Ordering},
    Arc, RwLock,
};

pub(crate) struct RecordedMessage {
    pub channel_id: ChannelId,
    pub message: Message,
    pub direction: ChannelDirection,
}

pub(crate) struct AppModel {
    runtime: Arc<NullableRuntime>,
    node_factory: NodeFactory,
    node: Option<Arc<Node>>,
    started: Arc<AtomicBool>,
    stopped: Arc<AtomicBool>,
    published: Arc<AtomicUsize>,
    inbound: Arc<AtomicUsize>,
    messages: Arc<RwLock<Vec<RecordedMessage>>>,
    selected: Option<MessageDetailsModel>,
}

impl AppModel {
    pub(crate) fn new(runtime: Arc<NullableRuntime>, node_builder_factory: NodeFactory) -> Self {
        Self {
            node_factory: node_builder_factory,
            runtime,
            node: None,
            started: Arc::new(AtomicBool::new(false)),
            stopped: Arc::new(AtomicBool::new(true)),
            published: Arc::new(AtomicUsize::new(0)),
            inbound: Arc::new(AtomicUsize::new(0)),
            messages: Arc::new(RwLock::new(Vec::new())),
            selected: None,
        }
    }

    pub(crate) fn with_runtime(runtime: tokio::runtime::Handle) -> Self {
        Self::new(
            Arc::new(NullableRuntime::new(runtime.clone())),
            NodeFactory::new(runtime),
        )
    }

    pub(crate) fn can_start_node(&self) -> bool {
        self.node.is_none() && self.stopped.load(Ordering::SeqCst)
    }

    pub(crate) fn can_stop_node(&self) -> bool {
        self.node.is_some() && self.started.load(Ordering::SeqCst)
    }

    pub(crate) fn start_beta_node(&mut self) {
        let published = self.published.clone();
        let inbound1 = self.inbound.clone();
        let inbound2 = self.inbound.clone();
        let messages1 = self.messages.clone();
        let messages2 = self.messages.clone();
        let messages3 = self.messages.clone();
        let callbacks = NodeCallbacks::builder()
            .on_publish(move |channel_id, message| {
                messages1.write().unwrap().push(RecordedMessage {
                    channel_id,
                    message: message.clone(),
                    direction: ChannelDirection::Outbound,
                });
                published.fetch_add(1, Ordering::SeqCst);
            })
            .on_inbound(move |channel_id, message| {
                messages2.write().unwrap().push(RecordedMessage {
                    channel_id,
                    message: message.clone(),
                    direction: ChannelDirection::Inbound,
                });
                inbound1.fetch_add(1, Ordering::SeqCst);
            })
            .on_inbound_dropped(move |channel_id, message| {
                messages3.write().unwrap().push(RecordedMessage {
                    channel_id,
                    message: message.clone(),
                    direction: ChannelDirection::Inbound,
                });
                inbound2.fetch_add(1, Ordering::SeqCst);
            })
            .finish();
        let node = self
            .node_factory
            .create_node(Networks::NanoBetaNetwork, callbacks);
        let node_l = node.clone();
        let started = self.started.clone();
        let stopped = self.stopped.clone();
        self.runtime.spawn_blocking(move || {
            node_l.start();
            started.store(true, Ordering::SeqCst);
            stopped.store(false, Ordering::SeqCst);
        });

        self.node = Some(node);
    }

    pub(crate) fn stop_node(&mut self) {
        if let Some(node) = self.node.take() {
            self.started.store(false, Ordering::SeqCst);
            {
                let stopped = self.stopped.clone();
                self.runtime.spawn_blocking(move || {
                    node.stop();
                    stopped.store(true, Ordering::SeqCst);
                });
            }
        }
    }

    pub(crate) fn status(&self) -> &'static str {
        if self.node.is_some() {
            if self.started.load(Ordering::SeqCst) {
                "running"
            } else {
                "starting..."
            }
        } else {
            if !self.stopped.load(Ordering::SeqCst) {
                "stopping..."
            } else {
                "not running"
            }
        }
    }

    pub(crate) fn messages_sent(&self) -> String {
        self.published
            .load(Ordering::SeqCst)
            .to_formatted_string(&Locale::en)
    }

    pub(crate) fn messages_received(&self) -> String {
        self.inbound
            .load(Ordering::SeqCst)
            .to_formatted_string(&Locale::en)
    }

    pub(crate) fn get_row(&self, index: usize) -> RowModel {
        let guard = self.messages.read().unwrap();
        let message = guard.get(index).unwrap();
        RowModel {
            channel_id: message.channel_id.to_string(),
            direction: if message.direction == ChannelDirection::Inbound {
                "in".into()
            } else {
                "out".into()
            },
            message: format!("{:?}", message.message.message_type()),
        }
    }

    pub(crate) fn message_count(&self) -> usize {
        self.messages.read().unwrap().len()
    }

    pub(crate) fn selected_message(&self) -> Option<MessageDetailsModel> {
        self.selected.clone()
    }

    pub(crate) fn select_message(&mut self, index: usize) {
        let msgs = self.messages.read().unwrap();
        self.selected = Some((&msgs[index]).into());
    }
}

#[derive(Clone)]
pub(crate) struct MessageDetailsModel {
    pub channel_id: String,
    pub direction: String,
    pub message_type: String,
    pub message: String,
}

impl From<&RecordedMessage> for MessageDetailsModel {
    fn from(value: &RecordedMessage) -> Self {
        Self {
            channel_id: value.channel_id.to_string(),
            direction: if value.direction == ChannelDirection::Inbound {
                "in".into()
            } else {
                "out".into()
            },
            message_type: format!("{:?}", value.message.message_type()),
            message: format!("{:#?}", value.message),
        }
    }
}

pub(crate) struct RowModel {
    pub channel_id: String,
    pub direction: String,
    pub message: String,
}

impl Default for AppModel {
    fn default() -> Self {
        Self::new(Arc::new(NullableRuntime::default()), NodeFactory::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn initial_status() {
        let model = AppModel::new(
            Arc::new(NullableRuntime::new_null()),
            NodeFactory::new_null(),
        );
        assert_eq!(model.can_start_node(), true);
        assert_eq!(model.can_stop_node(), false);
        assert_eq!(model.status(), "not running");
        assert_eq!(model.messages_sent(), "0");
    }

    #[tokio::test]
    async fn starting_node() {
        let runtime = Arc::new(NullableRuntime::new_null());
        let mut model = AppModel::new(runtime.clone(), NodeFactory::new_null());

        model.start_beta_node();

        assert_eq!(model.can_start_node(), false);
        assert_eq!(model.can_stop_node(), false);
        assert_eq!(model.status(), "starting...");
        assert_eq!(runtime.blocking_spawns(), 1);
    }

    #[tokio::test]
    async fn starting_completed() {
        let runtime = Arc::new(NullableRuntime::new_null());
        let mut model = AppModel::new(runtime.clone(), NodeFactory::new_null());
        model.start_beta_node();

        runtime.run_nulled_blocking_task();

        assert_eq!(model.status(), "running");
        assert_eq!(model.can_start_node(), false);
        assert_eq!(model.can_stop_node(), true);
    }

    #[tokio::test]
    async fn stopping_node() {
        let runtime = Arc::new(NullableRuntime::new_null());
        let mut model = AppModel::new(runtime.clone(), NodeFactory::new_null());
        model.start_beta_node();
        runtime.run_nulled_blocking_task();
        model.stop_node();
        assert_eq!(model.can_start_node(), false);
        assert_eq!(model.can_stop_node(), false);
        assert_eq!(model.status(), "stopping...");
        assert_eq!(runtime.blocking_spawns(), 2);
    }

    #[tokio::test]
    async fn stopping_completed() {
        let runtime = Arc::new(NullableRuntime::new_null());
        let mut model = AppModel::new(runtime.clone(), NodeFactory::new_null());
        model.start_beta_node();
        runtime.run_nulled_blocking_task();
        model.stop_node();
        runtime.run_nulled_blocking_task();
        assert_eq!(model.can_start_node(), true);
        assert_eq!(model.can_stop_node(), false);
        assert_eq!(model.status(), "not running");
    }
}
