use crate::{
    message_collection::{MessageCollection, RecordedMessage},
    message_rate_calculator::{MessageRates, MessageRatesCalculator},
};
use chrono::Utc;
use rsnano_network::ChannelDirection;
use rsnano_node::NodeCallbacks;
use rsnano_nullable_clock::{SteadyClock, Timestamp};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, RwLock,
};

pub(crate) struct MessageRecorder {
    pub rates: MessageRates,
    rate_calc: RwLock<MessageRatesCalculator>,
    messages: Arc<RwLock<MessageCollection>>,
    is_recording: AtomicBool,
}

impl MessageRecorder {
    pub(crate) fn new(messages: Arc<RwLock<MessageCollection>>) -> Self {
        Self {
            rates: Default::default(),
            rate_calc: RwLock::new(Default::default()),
            messages,
            is_recording: AtomicBool::new(false),
        }
    }

    pub fn is_recording(&self) -> bool {
        self.is_recording.load(Ordering::SeqCst)
    }

    pub fn start_recording(&self) {
        self.is_recording.store(true, Ordering::SeqCst);
    }

    pub fn stop_recording(&self) {
        self.is_recording.store(false, Ordering::SeqCst);
    }

    pub fn clear(&self) {
        self.messages.write().unwrap().clear();
    }

    pub fn record(&self, message: RecordedMessage, now: Timestamp) {
        {
            let mut rates = self.rate_calc.write().unwrap();
            rates.add(&message, now, &self.rates);
        }

        if self.is_recording() {
            let mut messages = self.messages.write().unwrap();
            messages.add(message);
        }
    }
}

pub(crate) fn make_node_callbacks(
    recorder: Arc<MessageRecorder>,
    clock: Arc<SteadyClock>,
) -> NodeCallbacks {
    let recorder2 = recorder.clone();
    let recorder3 = recorder.clone();
    let clock2 = clock.clone();
    let clock3 = clock.clone();
    NodeCallbacks::builder()
        .on_publish(move |channel_id, message| {
            let recorded = RecordedMessage {
                channel_id,
                message: message.clone(),
                direction: ChannelDirection::Outbound,
                date: Utc::now(),
            };
            recorder.record(recorded, clock.now());
        })
        .on_inbound(move |channel_id, message| {
            let recorded = RecordedMessage {
                channel_id,
                message: message.clone(),
                direction: ChannelDirection::Inbound,
                date: Utc::now(),
            };
            recorder2.record(recorded, clock2.now());
        })
        .on_inbound_dropped(move |channel_id, message| {
            let recorded = RecordedMessage {
                channel_id,
                message: message.clone(),
                direction: ChannelDirection::Inbound,
                date: Utc::now(),
            };
            recorder3.record(recorded, clock3.now());
        })
        .finish()
}
