use crate::TrafficType;
use std::sync::Arc;
use tokio::sync::{
    mpsc::{self},
    Notify,
};

pub struct WriteQueue {
    generic_queue: mpsc::Sender<Entry>,
    bootstrap_queue: mpsc::Sender<Entry>,
    notify_enqueued: Arc<Notify>,
    notify_dequeued: Arc<Notify>,
}

impl WriteQueue {
    pub fn new(max_size: usize) -> (Self, WriteQueueReceiver) {
        let (generic_tx, generic_rx) = mpsc::channel(max_size * 2);
        let (bootstrap_tx, bootstrap_rx) = mpsc::channel(max_size * 2);
        let notify_enqueued = Arc::new(Notify::new());
        let notify_dequeued = Arc::new(Notify::new());
        let receiver = WriteQueueReceiver {
            generic: generic_rx,
            bootstrap: bootstrap_rx,
            enqueued: notify_enqueued.clone(),
            dequeued: notify_dequeued.clone(),
        };
        (
            Self {
                generic_queue: generic_tx,
                bootstrap_queue: bootstrap_tx,
                notify_enqueued,
                notify_dequeued,
            },
            receiver,
        )
    }

    pub async fn insert(
        &self,
        buffer: Arc<Vec<u8>>,
        traffic_type: TrafficType,
    ) -> anyhow::Result<()> {
        let queue = self.queue_for(traffic_type);

        while queue.capacity() == 0 {
            self.notify_dequeued.notified().await;
        }

        let entry = Entry { buffer };
        queue
            .send(entry)
            .await
            .map_err(|_| anyhow!("queue closed"))?;
        self.notify_enqueued.notify_one();
        Ok(())
    }

    /// returns: inserted | write_error
    pub fn try_insert(&self, buffer: Arc<Vec<u8>>, traffic_type: TrafficType) -> (bool, bool) {
        let entry = Entry { buffer };
        let queue = self.queue_for(traffic_type);
        match queue.try_send(entry) {
            Ok(()) => {
                self.notify_enqueued.notify_one();
                (true, false)
            }
            Err(mpsc::error::TrySendError::Full(_)) => (false, false),
            Err(mpsc::error::TrySendError::Closed(_)) => (false, true),
        }
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

pub struct WriteQueueReceiver {
    generic: mpsc::Receiver<Entry>,
    bootstrap: mpsc::Receiver<Entry>,
    enqueued: Arc<Notify>,
    dequeued: Arc<Notify>,
}

impl WriteQueueReceiver {
    pub async fn pop(&mut self) -> Option<(Entry, TrafficType)> {
        // always prefer generic queue!
        if let Ok(result) = self.generic.try_recv() {
            return Some((result, TrafficType::Generic));
        }

        let result = tokio::select! {
            v = self.generic.recv() => v.map(|i| (i, TrafficType::Generic)),
            v = self.bootstrap.recv() => v.map(|i| (i, TrafficType::Bootstrap)),
        };

        if result.is_some() {
            self.dequeued.notify_one();
        }

        result
    }
}

pub struct Entry {
    pub buffer: Arc<Vec<u8>>,
}
