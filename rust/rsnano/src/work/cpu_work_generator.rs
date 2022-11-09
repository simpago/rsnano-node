use super::{WorkGenerator, WorkTicket, XorShift1024Star};
use crate::core::{Root, WorkVersion};
use blake2::{
    digest::{Update, VariableOutput},
    VarBlake2b,
};
use std::{mem::size_of, thread, time::Duration};

pub(crate) struct CpuWorkGenerator {
    // Quick RNG for work attempts.
    rng: XorShift1024Star,
    hasher: VarBlake2b,
    rate_limiter: Duration,
}

// Single threaded PoW generation on the CPU
impl CpuWorkGenerator {
    pub fn new(rate_limiter: Duration) -> Self {
        Self {
            rng: XorShift1024Star::new(),
            hasher: VarBlake2b::new_keyed(&[], size_of::<u64>()),
            rate_limiter,
        }
    }

    fn next(&mut self, item: &Root) -> (u64, u64) {
        let work = self.rng.next();
        let mut difficulty = 0;
        self.hasher.update(&work.to_le_bytes());
        self.hasher.update(item.as_bytes());
        self.hasher.finalize_variable_reset(|result| {
            difficulty = u64::from_le_bytes(result.try_into().unwrap());
        });
        (work, difficulty)
    }

    /// Tries to create PoW in a batch of 256 iterations
    fn try_create_batch(&mut self, item: &Root, min_difficulty: u64) -> Option<(u64, u64)> {
        // Don't query main memory every iteration in order to reduce memory bus traffic
        // All operations here operate on stack memory
        // Count iterations down to zero since comparing to zero is easier than comparing to another number
        let mut iteration = 256u32;
        let mut work = 0;
        let mut difficulty = 0;
        while iteration > 0 && difficulty < min_difficulty {
            (work, difficulty) = self.next(&item);
            iteration -= 1;
        }

        if difficulty >= min_difficulty {
            Some((work, difficulty))
        } else {
            None
        }
    }
}

impl WorkGenerator for CpuWorkGenerator {
    fn create(
        &mut self,
        _version: WorkVersion,
        item: &Root,
        min_difficulty: u64,
        work_ticket: &WorkTicket,
    ) -> Option<(u64, u64)> {
        while !work_ticket.expired() {
            let result = self.try_create_batch(item, min_difficulty);
            if result.is_some() {
                return result;
            }

            // Add a rate limiter (if specified) to the pow calculation to save some CPUs which don't want to operate at full throttle
            if !self.rate_limiter.is_zero() {
                thread::sleep(self.rate_limiter);
            }
        }
        None
    }
}
