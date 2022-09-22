use std::{sync::Mutex, time::SystemTime};

/// Histogram bin with interval, current value and timestamp of last update
#[derive(Clone)]
pub struct HistogramBin {
    pub start_inclusive: u64,
    pub end_exclusive: u64,
    pub value: u64,
    pub timestamp: SystemTime,
}

impl HistogramBin {
    pub fn new(start_inclusive: u64, end_exclusive: u64) -> Self {
        Self {
            start_inclusive,
            end_exclusive,
            value: 0,
            timestamp: SystemTime::now(),
        }
    }
}

/// Histogram values
pub struct StatHistogram {
    bins: Mutex<Vec<HistogramBin>>,
}

impl StatHistogram {
    /// Create histogram given a set of intervals and an optional bin count
    /// # Arguments
    /// * `intervals` - Inclusive-exclusive intervals, e.g. {1,5,8,15} produces bins [1,4] [5,7] [8, 14]
    /// * `bin_count` -  If zero (default), \p intervals_a defines all the bins. If non-zero, \p intervals_a contains the total range, which is uniformly distributed into \p bin_count_a bins.
    pub fn new(intervals: &[u64], bin_count: u64) -> Self {
        let mut bins = Vec::new();
        if bin_count == 0 {
            debug_assert!(intervals.len() > 1);
            let mut start_inclusive = intervals[0];
            for &i in &intervals[1..] {
                let end_exclusive = i;
                bins.push(HistogramBin::new(start_inclusive, end_exclusive));
                start_inclusive = end_exclusive;
            }
        } else {
            debug_assert!(intervals.len() == 2);
            let min_inclusive = intervals[0];
            let max_exclusive = intervals[1];
            let domain = max_exclusive - min_inclusive;
            let bin_size = (domain + bin_count - 1) / bin_count;
            let last_bin_size = domain % bin_size;
            let mut next_start = min_inclusive;

            for _ in 0..bin_count {
                bins.push(HistogramBin::new(next_start, next_start + bin_size));
                next_start += bin_size;
            }

            if last_bin_size > 0 {
                bins.push(HistogramBin::new(next_start, next_start + last_bin_size));
            }
        }

        Self {
            bins: Mutex::new(bins),
        }
    }

    /// Add `addend` to the histogram bin into which `index` falls
    pub fn add(&self, index: u64, addend: u64) {
        let mut lock = self.bins.lock().unwrap();
        debug_assert!(!lock.is_empty());

        // The search for a bin is linear, but we're searching just a few
        // contiguous items which are likely to be in cache.
        let mut found = false;
        for bin in lock.iter_mut() {
            if index >= bin.start_inclusive && index < bin.end_exclusive {
                bin.value += addend;
                bin.timestamp = SystemTime::now();
                found = true;
                break;
            }
        }

        // Clamp into first or last bin if no suitable bin was found
        if !found {
            if index < lock[0].start_inclusive {
                lock[0].value += addend;
            } else {
                lock.last_mut().unwrap().value += addend;
            }
        }
    }

    pub fn get_bins(&self) -> Vec<HistogramBin> {
        self.bins.lock().unwrap().clone()
    }
}

impl Clone for StatHistogram {
    fn clone(&self) -> Self {
        let lock = self.bins.lock().unwrap();
        Self {
            bins: Mutex::new(lock.clone()),
        }
    }
}
