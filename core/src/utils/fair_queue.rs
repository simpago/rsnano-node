use std::{
    cmp::min,
    collections::{BTreeMap, HashMap, VecDeque},
    hash::Hash,
    ops::RangeBounds,
};

use super::ContainerInfo;

/// Queue items of type T from source S
pub struct FairQueue<S, T>
where
    S: Ord + Copy,
{
    queues: BTreeMap<S, Entry<T>>,
    current_queue_key: Option<S>,
    max_size_query: Box<dyn Fn(&S) -> usize + Send + Sync>,
    priority_query: Box<dyn Fn(&S) -> usize + Send + Sync>,
    counter: usize,
    total_len: usize,
}

impl<S, T> FairQueue<S, T>
where
    S: Ord + Copy,
{
    pub fn new(
        max_size_query: impl Fn(&S) -> usize + Send + Sync + 'static,
        priority_query: impl Fn(&S) -> usize + Send + Sync + 'static,
    ) -> Self {
        Self {
            queues: BTreeMap::new(),
            current_queue_key: None,
            counter: 0,
            total_len: 0,
            max_size_query: Box::new(max_size_query),
            priority_query: Box::new(priority_query),
        }
    }

    pub fn sum_queue_len<R>(&self, range: R) -> usize
    where
        R: RangeBounds<S>,
    {
        self.queues.range(range).map(|(_, q)| q.len()).sum()
    }

    pub fn queue_len(&self, source: &S) -> usize {
        self.queues.get(source).map(|q| q.len()).unwrap_or_default()
    }

    pub fn max_len(&self, source: &S) -> usize {
        self.queues
            .get(source)
            .map(|q| q.max_size)
            .unwrap_or_default()
    }

    pub fn priority(&self, source: &S) -> usize {
        self.queues
            .get(source)
            .map(|q| q.priority)
            .unwrap_or_default()
    }

    pub fn free_capacity(&self, source: &S) -> usize {
        self.queues
            .get(source)
            .map(|q| q.max_size - q.len())
            .unwrap_or_else(|| (self.max_size_query)(source))
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

    /// Push an item to the appropriate queue based on the source
    /// item will be dropped if the queue is full
    /// @return true if added, false if dropped
    pub fn push(&mut self, source: S, item: T) -> bool {
        let entry = self.queues.entry(source.clone()).or_insert_with(|| {
            let max_size = (self.max_size_query)(&source);
            let priority = (self.priority_query)(&source);
            Entry::new(max_size, priority)
        });
        let added = entry.push(item);
        if added {
            self.total_len += 1;
        }
        added
    }

    pub fn next(&mut self) -> Option<(S, T)> {
        if self.total_len == 0 {
            return None;
        }

        if self.should_seek() {
            self.seek_next();
        }

        let it = self.current_queue_key.as_ref()?;
        let queue = self.queues.get_mut(it).unwrap();
        self.counter += 1;
        self.total_len -= 1;
        Some((it.clone(), queue.pop().unwrap()))
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

    pub fn next_batch(&mut self, max_count: usize) -> VecDeque<(S, T)> {
        let count = min(self.len(), max_count);

        let mut result = VecDeque::new();
        while result.len() < count {
            result.push_back(self.next().unwrap());
        }
        result
    }

    pub fn remove(&mut self, key: &S) {
        if let Some(removed) = self.queues.remove(key) {
            self.total_len -= removed.len();
            self.current_queue_key = None;
        }
    }

    pub fn compacted_info<R>(&self, compact: impl Fn(&S) -> R) -> FairQueueInfo<R>
    where
        R: Hash + Eq + Copy,
    {
        let mut total_size = 0;
        let mut total_max_size = 0;
        let mut queues_info: HashMap<R, QueueInfo<R>> = HashMap::new();

        // Iterate over all queues
        for (key, entry) in self.queues.iter() {
            let compacted_source = compact(key);
            let size = entry.len();
            total_size += size;
            total_max_size += entry.max_size;

            queues_info
                .entry(compacted_source)
                .and_modify(|i| {
                    i.size += size;
                    i.max_size += entry.max_size;
                })
                .or_insert_with(|| QueueInfo {
                    source: compacted_source,
                    size,
                    max_size: entry.max_size,
                });
        }

        FairQueueInfo {
            queues: queues_info,
            total_size,
            total_max_size,
        }
    }

    pub fn container_info(&self) -> ContainerInfo {
        [
            (
                "queues",
                self.queues.len(),
                std::mem::size_of::<S>() + std::mem::size_of::<Entry<T>>(),
            ),
            (
                "total_size",
                self.len(),
                std::mem::size_of::<S>() + std::mem::size_of::<Entry<T>>(),
            ),
        ]
        .into()
    }

    fn seek_next(&mut self) {
        self.counter = 0;
        //TODO unwraps and inefficient access!
        //TODO Endless loop if everything is empty!
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
}

struct Entry<T> {
    requests: VecDeque<T>,
    priority: usize,
    max_size: usize,
}

impl<T> Entry<T> {
    pub fn new(max_size: usize, priority: usize) -> Self {
        Self {
            max_size,
            priority,
            requests: Default::default(),
        }
    }

    pub fn pop(&mut self) -> Option<T> {
        self.requests.pop_front()
    }

    pub fn push(&mut self, request: T) -> bool {
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

pub struct FairQueueInfo<S>
where
    S: Clone + Hash + Eq,
{
    pub queues: HashMap<S, QueueInfo<S>>,
    pub total_size: usize,
    pub total_max_size: usize,
}

impl<S> Default for FairQueueInfo<S>
where
    S: Clone + Hash + Eq,
{
    fn default() -> Self {
        Self {
            total_size: 0,
            total_max_size: 0,
            queues: Default::default(),
        }
    }
}

#[derive(Clone)]
pub struct QueueInfo<S>
where
    S: Clone,
{
    pub source: S,
    pub size: usize,
    pub max_size: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty() {
        let queue: FairQueue<usize, &'static str> = FairQueue::new(|_| 999, |_| 999);
        assert_eq!(queue.len(), 0);
        assert!(queue.is_empty());
        assert_eq!(queue.free_capacity(&1), 999);
        assert_eq!(queue.free_capacity(&2), 999);
    }

    #[test]
    fn process_one() {
        let mut queue: FairQueue<usize, &'static str> = FairQueue::new(|_| 1, |_| 1);
        queue.push(7, "foo");

        assert_eq!(queue.len(), 1);
        assert_eq!(queue.queues_len(), 1);
        assert_eq!(queue.queue_len(&7), 1);
        assert_eq!(queue.queue_len(&8), 0);
        assert_eq!(queue.free_capacity(&7), 0);
        assert_eq!(queue.free_capacity(&8), 1);

        let (source, item) = queue.next().unwrap();
        assert_eq!(source, 7);
        assert_eq!(item, "foo");
        assert!(queue.is_empty());
    }

    #[test]
    fn fifo() {
        let mut queue: FairQueue<usize, &'static str> = FairQueue::new(|_| 999, |_| 1);

        queue.push(7, "a");
        queue.push(7, "b");
        queue.push(7, "c");

        assert_eq!(queue.len(), 3);
        assert_eq!(queue.queues_len(), 1);
        assert_eq!(queue.queue_len(&7), 3);

        assert_eq!(queue.next(), Some((7, "a")));
        assert_eq!(queue.next(), Some((7, "b")));
        assert_eq!(queue.next(), Some((7, "c")));
        assert!(queue.is_empty());
        assert_eq!(queue.free_capacity(&7), 999);
    }

    #[test]
    fn process_many() {
        let mut queue: FairQueue<usize, &'static str> = FairQueue::new(|_| 1, |_| 1);

        queue.push(7, "a");
        queue.push(8, "b");
        queue.push(9, "c");

        assert_eq!(queue.len(), 3);
        assert_eq!(queue.queues_len(), 3);

        assert_eq!(queue.next(), Some((7, "a")));
        assert_eq!(queue.next(), Some((8, "b")));
        assert_eq!(queue.next(), Some((9, "c")));
        assert!(queue.is_empty());
    }

    #[test]
    fn max_queue_size() {
        let mut queue: FairQueue<usize, &'static str> = FairQueue::new(|_| 2, |_| 1);

        queue.push(7, "a");
        queue.push(7, "b");
        queue.push(7, "c");

        assert_eq!(queue.len(), 2);

        assert_eq!(queue.next(), Some((7, "a")));
        assert_eq!(queue.next(), Some((7, "b")));
        assert!(queue.is_empty());
    }

    #[test]
    fn round_robin_with_priority() {
        let mut queue: FairQueue<usize, &'static str> = FairQueue::new(
            |_| 999,
            |origin| match origin {
                7 => 1,
                8 => 2,
                9 => 3,
                _ => unreachable!(),
            },
        );

        queue.push(7, "7a");
        queue.push(7, "7b");
        queue.push(7, "7c");
        queue.push(8, "8a");
        queue.push(8, "8b");
        queue.push(8, "8c");
        queue.push(9, "9a");
        queue.push(9, "9b");
        queue.push(9, "9c");
        assert_eq!(queue.len(), 9);

        // Processing 1x live, 2x bootstrap, 3x unchecked before moving to the next source
        assert_eq!(queue.next().unwrap().1, "7a");
        assert_eq!(queue.next().unwrap().1, "8a");
        assert_eq!(queue.next().unwrap().1, "8b");
        assert_eq!(queue.next().unwrap().1, "9a");
        assert_eq!(queue.next().unwrap().1, "9b");
        assert_eq!(queue.next().unwrap().1, "9c");
        assert_eq!(queue.next().unwrap().1, "7b");
        assert_eq!(queue.next().unwrap().1, "8c");
        assert_eq!(queue.next().unwrap().1, "7c");
        assert!(queue.is_empty());
    }

    #[test]
    fn sum_queue_len() {
        let mut queue: FairQueue<usize, &'static str> = FairQueue::new(|_| 999, |_| 999);

        queue.push(3, "x");
        queue.push(4, "x");
        queue.push(4, "x");
        queue.push(5, "x");
        queue.push(5, "x");
        queue.push(6, "x");
        queue.push(7, "x");

        assert_eq!(queue.sum_queue_len(4..=6), 5);
    }
}
