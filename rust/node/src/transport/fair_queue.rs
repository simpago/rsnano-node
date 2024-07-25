use super::ChannelEnum;
use rsnano_core::utils::{ContainerInfo, ContainerInfoComponent};
use std::{
    cmp::{min, Ordering},
    collections::{BTreeMap, VecDeque},
    sync::Arc,
    time::{Duration, Instant},
};

/// Holds user supplied source type(s) and an optional channel.
/// This is used to uniquely identify and categorize the source of a request.
#[derive(Clone)]
pub struct Origin<S>
where
    S: Ord + Copy,
{
    pub source: S,

    /// This can be null for some sources (eg. local RPC) to indicate that the source is not associated with a channel.
    pub channel: Option<Arc<ChannelEnum>>,
}

impl<S> Origin<S>
where
    S: Ord + Copy,
{
    pub fn new(source: S, channel: Arc<ChannelEnum>) -> Self {
        Self {
            source,
            channel: Some(channel),
        }
    }

    pub fn new_opt(source: S, channel: Option<Arc<ChannelEnum>>) -> Self {
        Self { source, channel }
    }

    pub fn is_alive(&self) -> bool {
        if let Some(channel) = &self.channel {
            channel.is_alive()
        } else {
            // Some sources (eg. local RPC) don't have an associated channel, never remove their queue
            true
        }
    }
}

impl<S> PartialEq for Origin<S>
where
    S: Ord + Copy,
{
    fn eq(&self, other: &Self) -> bool {
        matches!(self.cmp(&other), Ordering::Equal)
    }
}

impl<S> Eq for Origin<S> where S: Ord + Copy {}

impl<S> Ord for Origin<S>
where
    S: Ord + Copy,
{
    fn cmp(&self, other: &Self) -> Ordering {
        let source_ordering = self.source.cmp(&other.source);
        if !matches!(source_ordering, std::cmp::Ordering::Equal) {
            return source_ordering;
        }

        match (self.channel.as_ref(), other.channel.as_ref()) {
            (None, None) => Ordering::Equal,
            (Some(c1), Some(c2)) => Arc::as_ptr(c1).cmp(&Arc::as_ptr(c2)),
            (Some(_), None) => Ordering::Greater,
            (None, Some(_)) => Ordering::Less,
        }
    }
}

impl<S> PartialOrd for Origin<S>
where
    S: Ord + Copy,
{
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<S> From<S> for Origin<S>
where
    S: Ord + Copy,
{
    fn from(value: S) -> Self {
        Origin {
            source: value,
            channel: None,
        }
    }
}

struct Entry<R> {
    requests: VecDeque<R>,
    priority: usize,
    max_size: usize,
}

impl<R> Entry<R> {
    pub fn new(max_size: usize, priority: usize) -> Self {
        Self {
            max_size,
            priority,
            requests: Default::default(),
        }
    }

    pub fn pop(&mut self) -> Option<R> {
        self.requests.pop_front()
    }

    pub fn push(&mut self, request: R) -> bool {
        if self.requests.len() < self.max_size {
            self.requests.push_back(request);
            true // Added
        } else {
            false // Dropped
        }
    }

    pub fn is_empty(&self) -> bool {
        self.requests.is_empty()
    }

    pub fn len(&self) -> usize {
        self.requests.len()
    }
}

pub struct FairQueue<R, S>
where
    S: Ord + Copy,
{
    queues: BTreeMap<Origin<S>, Entry<R>>,
    last_update: Instant,
    current_queue_key: Option<Origin<S>>,
    max_size_query: Box<dyn Fn(&Origin<S>) -> usize + Send + Sync>,
    priority_query: Box<dyn Fn(&Origin<S>) -> usize + Send + Sync>,
    counter: usize,
    total_len: usize,
}

impl<R, S> FairQueue<R, S>
where
    S: Ord + Copy,
{
    pub fn new(
        max_size_query: Box<dyn Fn(&Origin<S>) -> usize + Send + Sync>,
        priority_query: Box<dyn Fn(&Origin<S>) -> usize + Send + Sync>,
    ) -> Self {
        Self {
            queues: BTreeMap::new(),
            last_update: Instant::now(),
            current_queue_key: None,
            counter: 0,
            total_len: 0,
            max_size_query,
            priority_query,
        }
    }

    pub fn queue_len(&self, source: &Origin<S>) -> usize {
        self.queues.get(source).map(|q| q.len()).unwrap_or_default()
    }

    pub fn max_len(&self, source: &Origin<S>) -> usize {
        self.queues
            .get(source)
            .map(|q| q.max_size)
            .unwrap_or_default()
    }

    pub fn priority(&self, source: &Origin<S>) -> usize {
        self.queues
            .get(source)
            .map(|q| q.priority)
            .unwrap_or_default()
    }

    pub fn len(&self) -> usize {
        self.total_len
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn queues_len(&self) -> usize {
        self.queues.len()
    }

    pub fn clear(&mut self) {
        self.queues.clear();
    }

    /// Should be called periodically to clean up stale channels and update queue priorities and max sizes
    pub fn periodic_update(&mut self, interval: Duration) -> bool {
        if self.last_update.elapsed() < interval {
            return false; // Not updated
        }
        self.last_update = Instant::now();
        self.cleanup();
        self.update();
        true // Updated
    }

    /// Push a request to the appropriate queue based on the source
    /// Request will be dropped if the queue is full
    /// @return true if added, false if dropped
    pub fn push(&mut self, request: R, source: Origin<S>) -> bool {
        let entry = self.queues.entry(source.clone()).or_insert_with(|| {
            let max_size = (self.max_size_query)(&source);
            let priority = (self.priority_query)(&source);
            Entry::new(max_size, priority)
        });
        let added = entry.push(request);
        if added {
            self.total_len += 1;
        }
        added
    }

    pub fn next(&mut self) -> Option<(R, Origin<S>)> {
        if self.should_seek() {
            self.seek_next();
        }

        let it = self.current_queue_key.as_ref()?;
        let queue = self.queues.get_mut(it).unwrap();
        self.counter += 1;
        self.total_len -= 1;
        Some((queue.pop().unwrap(), it.clone()))
    }

    fn should_seek(&self) -> bool {
        match &self.current_queue_key {
            Some(key) => match self.queues.get(key) {
                Some(queue) => {
                    if queue.is_empty() {
                        true
                    } else {
                        // Allow up to `queue.priority` requests to be processed before moving to the next queue
                        self.counter >= queue.priority
                    }
                }
                None => true,
            },
            None => true,
        }
    }

    pub fn next_batch(&mut self, max_count: usize) -> VecDeque<(R, Origin<S>)> {
        self.periodic_update(Duration::from_secs(30));
        let count = min(self.len(), max_count);

        let mut result = VecDeque::new();
        while result.len() < count {
            result.push_back(self.next().unwrap());
        }
        result
    }

    pub fn collect_container_info(&self, name: impl Into<String>) -> ContainerInfoComponent {
        ContainerInfoComponent::Composite(
            name.into(),
            vec![
                ContainerInfoComponent::Leaf(ContainerInfo {
                    name: "queues".to_string(),
                    count: self.queues.len(),
                    sizeof_element: std::mem::size_of::<Origin<S>>()
                        + std::mem::size_of::<Entry<R>>(),
                }),
                ContainerInfoComponent::Leaf(ContainerInfo {
                    name: "total_size".to_string(),
                    count: self.len(),
                    sizeof_element: std::mem::size_of::<Origin<S>>()
                        + std::mem::size_of::<Entry<R>>(),
                }),
            ],
        )
    }

    fn seek_next(&mut self) {
        self.counter = 0;
        //TODO unwraps and inefficient access! Endless loop?
        loop {
            if let Some(current) = self.current_queue_key.take() {
                let mut it = self.queues.range(current..);
                if let Some(_) = it.next() {
                    self.current_queue_key = it.next().map(|(k, _)| k.clone());
                }
            }

            if self.current_queue_key.is_none() {
                self.current_queue_key = Some(self.queues.first_key_value().unwrap().0.clone());
            }

            if !self
                .queues
                .get(self.current_queue_key.as_ref().unwrap())
                .unwrap()
                .is_empty()
            {
                break;
            }
        }
    }

    fn cleanup(&mut self) {
        self.current_queue_key = None;
        // Only removing empty queues, no need to update the `total size` counter
        self.queues.retain(|k, v| k.is_alive() || !v.is_empty());
    }

    fn update(&mut self) {
        for (source, queue) in self.queues.iter_mut() {
            queue.max_size = (self.max_size_query)(source);
            queue.priority = (self.priority_query)(source);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
    enum TestSource {
        Live,
        Bootstrap,
        Unchecked,
    }

    #[test]
    fn empty() {
        let queue: FairQueue<i32, TestSource> =
            FairQueue::new(Box::new(|_| 999), Box::new(|_| 999));
        assert_eq!(queue.len(), 0);
        assert!(queue.is_empty());
    }

    #[test]
    fn process_one() {
        let mut queue: FairQueue<i32, TestSource> =
            FairQueue::new(Box::new(|_| 1), Box::new(|_| 1));
        queue.push(7, TestSource::Live.into());

        assert_eq!(queue.len(), 1);
        assert_eq!(queue.queues_len(), 1);
        assert_eq!(queue.queue_len(&TestSource::Live.into()), 1);
        assert_eq!(queue.queue_len(&TestSource::Bootstrap.into()), 0);

        let (result, origin) = queue.next().unwrap();
        assert_eq!(result, 7);
        assert_eq!(origin.source, TestSource::Live);
        assert!(origin.channel.is_none());
        assert!(queue.is_empty());
    }

    #[test]
    fn fifo() {
        let mut queue: FairQueue<i32, TestSource> =
            FairQueue::new(Box::new(|_| 999), Box::new(|_| 1));

        queue.push(7, TestSource::Live.into());
        queue.push(8, TestSource::Live.into());
        queue.push(9, TestSource::Live.into());

        assert_eq!(queue.len(), 3);
        assert_eq!(queue.queues_len(), 1);
        assert_eq!(queue.queue_len(&TestSource::Live.into()), 3);

        let (result, origin) = queue.next().unwrap();
        assert_eq!(result, 7);
        assert_eq!(origin.source, TestSource::Live);

        let (result, origin) = queue.next().unwrap();
        assert_eq!(result, 8);
        assert_eq!(origin.source, TestSource::Live);

        let (result, origin) = queue.next().unwrap();
        assert_eq!(result, 9);
        assert_eq!(origin.source, TestSource::Live);

        assert!(queue.is_empty());
    }

    #[test]
    fn process_many() {
        let mut queue: FairQueue<i32, TestSource> =
            FairQueue::new(Box::new(|_| 1), Box::new(|_| 1));

        queue.push(7, TestSource::Live.into());
        queue.push(8, TestSource::Bootstrap.into());
        queue.push(9, TestSource::Unchecked.into());

        assert_eq!(queue.len(), 3);
        assert_eq!(queue.queues_len(), 3);

        let (result, origin) = queue.next().unwrap();
        assert_eq!(result, 7);
        assert_eq!(origin.source, TestSource::Live);

        let (result, origin) = queue.next().unwrap();
        assert_eq!(result, 8);
        assert_eq!(origin.source, TestSource::Bootstrap);

        let (result, origin) = queue.next().unwrap();
        assert_eq!(result, 9);
        assert_eq!(origin.source, TestSource::Unchecked);

        assert!(queue.is_empty());
    }

    #[test]
    fn max_queue_size() {
        let mut queue: FairQueue<i32, TestSource> =
            FairQueue::new(Box::new(|_| 2), Box::new(|_| 1));

        queue.push(7, TestSource::Live.into());
        queue.push(8, TestSource::Live.into());
        queue.push(9, TestSource::Live.into());

        assert_eq!(queue.len(), 2);
        assert_eq!(queue.queues_len(), 1);
        assert_eq!(queue.queue_len(&TestSource::Live.into()), 2);

        let (result, origin) = queue.next().unwrap();
        assert_eq!(result, 7);
        assert_eq!(origin.source, TestSource::Live);

        let (result, origin) = queue.next().unwrap();
        assert_eq!(result, 8);
        assert_eq!(origin.source, TestSource::Live);

        assert!(queue.is_empty());
    }

    #[test]
    fn round_robin_with_priority() {
        let mut queue: FairQueue<i32, TestSource> = FairQueue::new(
            Box::new(|_| 999),
            Box::new(|origin| match origin.source {
                TestSource::Live => 1,
                TestSource::Bootstrap => 2,
                TestSource::Unchecked => 3,
            }),
        );

        queue.push(7, TestSource::Live.into());
        queue.push(8, TestSource::Live.into());
        queue.push(9, TestSource::Live.into());
        queue.push(10, TestSource::Bootstrap.into());
        queue.push(11, TestSource::Bootstrap.into());
        queue.push(12, TestSource::Bootstrap.into());
        queue.push(13, TestSource::Unchecked.into());
        queue.push(14, TestSource::Unchecked.into());
        queue.push(15, TestSource::Unchecked.into());
        assert_eq!(queue.len(), 9);

        // Processing 1x live, 2x bootstrap, 3x unchecked before moving to the next source
        assert_eq!(queue.next().unwrap().1.source, TestSource::Live);
        assert_eq!(queue.next().unwrap().1.source, TestSource::Bootstrap);
        assert_eq!(queue.next().unwrap().1.source, TestSource::Bootstrap);
        assert_eq!(queue.next().unwrap().1.source, TestSource::Unchecked);
        assert_eq!(queue.next().unwrap().1.source, TestSource::Unchecked);
        assert_eq!(queue.next().unwrap().1.source, TestSource::Unchecked);
        assert_eq!(queue.next().unwrap().1.source, TestSource::Live);
        assert_eq!(queue.next().unwrap().1.source, TestSource::Bootstrap);
        assert_eq!(queue.next().unwrap().1.source, TestSource::Live);
        assert!(queue.is_empty());
    }

    #[test]
    fn source_channel() {
        let mut queue: FairQueue<i32, TestSource> =
            FairQueue::new(Box::new(|_| 999), Box::new(|_| 1));

        let channel1 = Arc::new(ChannelEnum::new_null());
        let channel2 = Arc::new(ChannelEnum::new_null());
        let channel3 = Arc::new(ChannelEnum::new_null());

        queue.push(6, Origin::new(TestSource::Live, Arc::clone(&channel1)));
        queue.push(7, Origin::new(TestSource::Live, Arc::clone(&channel2)));
        queue.push(8, Origin::new(TestSource::Live, Arc::clone(&channel3)));
        queue.push(9, Origin::new(TestSource::Live, Arc::clone(&channel1))); // Channel 1 has multiple entries
        assert_eq!(queue.len(), 4);
        assert_eq!(queue.queues_len(), 3); // Each <source, channel> pair is a separate queue
        assert_eq!(
            queue.queue_len(&Origin::new(TestSource::Live, Arc::clone(&channel1))),
            2
        );
        assert_eq!(queue.queue_len(&Origin::new(TestSource::Live, channel2)), 1);
        assert_eq!(queue.queue_len(&Origin::new(TestSource::Live, channel3)), 1);

        let all = queue.next_batch(999);
        assert_eq!(all.len(), 4);

        let _channel1_results = all
            .iter()
            .filter(|i| Arc::ptr_eq(i.1.channel.as_ref().unwrap(), &channel1));
        assert!(queue.is_empty());
    }

    #[test]
    fn cleanup() {
        let mut queue: FairQueue<i32, TestSource> =
            FairQueue::new(Box::new(|_| 999), Box::new(|_| 1));

        let channel1 = Arc::new(ChannelEnum::new_null());
        let channel2 = Arc::new(ChannelEnum::new_null());
        let channel3 = Arc::new(ChannelEnum::new_null());

        queue.push(7, Origin::new(TestSource::Live, Arc::clone(&channel1)));
        queue.push(8, Origin::new(TestSource::Live, Arc::clone(&channel2)));
        queue.push(9, Origin::new(TestSource::Live, Arc::clone(&channel3)));

        // Only closing the channel should make it eligable for cleanup
        channel1.close();
        drop(channel2);

        assert!(queue.periodic_update(Duration::ZERO));

        // Until the queue is drained, the entries are still present
        assert_eq!(queue.len(), 3);
        assert_eq!(queue.queues_len(), 3);

        queue.next_batch(999);
        assert!(queue.periodic_update(Duration::ZERO));

        assert!(queue.is_empty());
        assert_eq!(queue.queues_len(), 2);
    }
}
