use super::TrafficType;
use crate::utils::ErrorCode;
use std::sync::Arc;
use tokio::sync::mpsc::{self};

pub(crate) struct WriteQueue {
    generic_queue: mpsc::Sender<Entry>,
    bootstrap_queue: mpsc::Sender<Entry>,
}

impl WriteQueue {
    pub fn new(max_size: usize) -> (Self, WriteQueueReceiver) {
        let (generic_tx, generic_rx) = mpsc::channel(max_size * 2);
        let (bootstrap_tx, bootstrap_rx) = mpsc::channel(max_size * 2);
        let receiver = WriteQueueReceiver::new(generic_rx, bootstrap_rx);
        (
            Self {
                generic_queue: generic_tx,
                bootstrap_queue: bootstrap_tx,
            },
            receiver,
        )
    }

    pub async fn insert(
        &self,
        buffer: Arc<Vec<u8>>,
        traffic_type: TrafficType,
    ) -> anyhow::Result<()> {
        let entry = Entry {
            buffer,
            callback: None,
        };
        self.queue_for(traffic_type)
            .send(entry)
            .await
            .map_err(|_| anyhow!("queue closed"))
    }

    /// returns: inserted | write_error | callback
    pub fn try_insert(
        &self,
        buffer: Arc<Vec<u8>>,
        callback: Option<WriteCallback>,
        traffic_type: TrafficType,
    ) -> (bool, bool, Option<WriteCallback>) {
        let entry = Entry { buffer, callback };
        match self.queue_for(traffic_type).try_send(entry) {
            Ok(()) => (true, false, None),
            Err(mpsc::error::TrySendError::Full(e)) => (false, false, e.callback),
            Err(mpsc::error::TrySendError::Closed(e)) => (false, true, e.callback),
        }
    }

    pub fn is_closed(&self) -> bool {
        self.generic_queue.is_closed() || self.bootstrap_queue.is_closed()
    }

    pub fn capacity(&self, traffic_type: TrafficType) -> usize {
        self.queue_for(traffic_type).capacity()
    }

    fn queue_for(&self, traffic_type: TrafficType) -> &mpsc::Sender<Entry> {
        match traffic_type {
            TrafficType::Generic => &self.generic_queue,
            TrafficType::Bootstrap => &self.bootstrap_queue,
        }
    }
}

pub(crate) struct WriteQueueReceiver {
    generic: mpsc::Receiver<Entry>,
    bootstrap: mpsc::Receiver<Entry>,
}

impl WriteQueueReceiver {
    fn new(generic: mpsc::Receiver<Entry>, bootstrap: mpsc::Receiver<Entry>) -> Self {
        Self { generic, bootstrap }
    }

    pub(crate) async fn pop(&mut self) -> Option<Entry> {
        // always prefer generic queue!
        if let Ok(result) = self.generic.try_recv() {
            return Some(result);
        }

        tokio::select! {
            v = self.generic.recv() => v,
            v = self.bootstrap.recv() => v,
        }
    }
}

pub type WriteCallback = Box<dyn FnOnce(ErrorCode, usize) + Send>;

pub(crate) struct Entry {
    pub buffer: Arc<Vec<u8>>,
    pub callback: Option<WriteCallback>,
}
