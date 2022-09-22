use anyhow::Result;
use scoped_threadpool::Pool;

use crate::{validate_message_batch, PublicKey, Signature};
use std::{
    hint::spin_loop,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Mutex,
    },
    time::{Duration, Instant},
};
pub struct SignatureCheckSet {
    pub messages: Vec<Vec<u8>>,
    pub pub_keys: Vec<PublicKey>,
    pub signatures: Vec<Signature>,
    pub verifications: Vec<i32>,
}

pub struct SignatureCheckSetBatch<'a> {
    pub messages: &'a [Vec<u8>],
    pub pub_keys: &'a [PublicKey],
    pub signatures: &'a [Signature],
    pub verifications: &'a mut [i32],
}

impl SignatureCheckSet {
    pub fn new(
        messages: Vec<Vec<u8>>,
        pub_keys: Vec<PublicKey>,
        signatures: Vec<Signature>,
    ) -> Self {
        let size = messages.len();
        assert!(pub_keys.len() == size);
        assert!(signatures.len() == size);
        Self {
            messages,
            pub_keys,
            signatures,
            verifications: vec![-1; size],
        }
    }

    pub fn size(&self) -> usize {
        self.messages.len()
    }

    pub fn as_batch(&mut self) -> SignatureCheckSetBatch {
        SignatureCheckSetBatch {
            messages: &self.messages,
            pub_keys: &self.pub_keys,
            signatures: &self.signatures,
            verifications: &mut self.verifications,
        }
    }
}

pub struct SignatureChecker {
    // todo: scoped_threadpool is behind a Mutex which leads to unnecessary waiting in vote_processor and state_block_verification...
    // Ideally there should be no locking required when adding work to the threadpool!
    thread_pool: Mutex<Option<Pool>>,
    thread_pool_threads: usize,
    tasks_remaining: AtomicUsize,
    stopped: AtomicBool,
}

impl SignatureChecker {
    pub fn new(num_threads: usize) -> Self {
        Self {
            thread_pool: if num_threads == 0 {
                Mutex::new(None)
            } else {
                Mutex::new(Some(Pool::new(num_threads as u32)))
            },
            thread_pool_threads: num_threads,
            tasks_remaining: AtomicUsize::new(0),
            stopped: AtomicBool::new(false),
        }
    }

    pub const BATCH_SIZE: usize = 256;

    pub fn max_size(&self) -> usize {
        Self::BATCH_SIZE * (self.thread_pool_threads + 1)
    }

    pub fn flush(&self) -> Result<()> {
        let instant = Instant::now();
        while !self.stopped.load(Ordering::SeqCst)
            && self.tasks_remaining.load(Ordering::SeqCst) != 0
            && instant.elapsed() < Duration::from_secs(20)
        {
            spin_loop();
        }

        if instant.elapsed() >= Duration::from_secs(20) {
            Err(anyhow!("timeout in flush"))
        } else {
            Ok(())
        }
    }

    pub fn stop(&self) {
        self.stopped.swap(true, Ordering::SeqCst);
        drop(self.thread_pool.lock().unwrap().take());
    }

    pub fn verify(&self, check_set: &mut SignatureCheckSet) {
        if self.stopped.load(Ordering::SeqCst) {
            return;
        }

        if check_set.size() <= SignatureChecker::BATCH_SIZE {
            // Not dealing with many so just use the calling thread for checking signatures
            Self::verify_batch(&mut check_set.as_batch());
        } else {
            let mut pool = self.thread_pool.lock().unwrap();
            if let Some(pool) = &mut *pool {
                self.verify_batch_async(check_set, pool);
            } else {
                drop(pool);
                Self::verify_batch(&mut check_set.as_batch());
            }
        }
    }

    pub fn verify_batch(check_set: &mut SignatureCheckSetBatch) {
        validate_message_batch(
            check_set.messages,
            check_set.pub_keys,
            check_set.signatures,
            check_set.verifications,
        );

        let result = check_set.verifications.iter().all(|&x| x == 0 || x == 1);
        assert!(result);
    }

    fn verify_batch_async(&self, check_set: &mut SignatureCheckSet, pool: &mut Pool) {
        let thread_distribution_plan = ThreadDistributionPlan::new(
            check_set.size(),
            self.thread_pool_threads,
            Self::BATCH_SIZE,
        );

        let task_pending = AtomicUsize::new(thread_distribution_plan.thread_pool_batches);

        pool.scoped(|scope| {
            // Verify a number of signature batches over the thread pool (does not block)
            /* This operates on a number of signatures of size (num_batches * batch_size) from the beginning of the check_a pointers.
             */
            let split_index = thread_distribution_plan.thread_pool_checks();
            let (messages_pool, messages_calling) = check_set.messages.split_at(split_index);
            let (keys_pool, keys_calling) = check_set.pub_keys.split_at(split_index);
            let (signatures_pool, signatures_calling) = check_set.signatures.split_at(split_index);
            let (verify_pool, verify_calling) = check_set.verifications.split_at_mut(split_index);

            let task_pending = &task_pending;
            self.tasks_remaining.fetch_add(1, Ordering::SeqCst);
            let tasks_remaining = &self.tasks_remaining;

            let mut message_chunks = messages_pool.chunks(thread_distribution_plan.batch_size);
            let mut key_chunks = keys_pool.chunks(thread_distribution_plan.batch_size);
            let mut signature_chunks = signatures_pool.chunks(thread_distribution_plan.batch_size);
            let mut verify_chunks = verify_pool.chunks_mut(thread_distribution_plan.batch_size);
            for messages in message_chunks.by_ref() {
                let mut batch = SignatureCheckSetBatch {
                    messages,
                    pub_keys: key_chunks.next().unwrap(),
                    signatures: signature_chunks.next().unwrap(),
                    verifications: verify_chunks.next().unwrap(),
                };

                scope.execute(move || {
                    Self::verify_batch(&mut batch);
                    if task_pending.fetch_sub(1, Ordering::SeqCst) == 1 {
                        tasks_remaining.fetch_sub(1, Ordering::SeqCst);
                    }
                });
            }

            // Verify the rest on the calling thread, this operates on the signatures at the end of the check set
            let mut batch = SignatureCheckSetBatch {
                messages: messages_calling,
                pub_keys: keys_calling,
                signatures: signatures_calling,
                verifications: verify_calling,
            };
            Self::verify_batch(&mut batch);
        });
    }
}

impl Drop for SignatureChecker {
    fn drop(&mut self) {
        self.stop()
    }
}

/// Split up the tasks equally over the calling thread and the thread pool.
/// Any overflow on the modulus of the batch_size is given to the calling thread, so the thread pool
/// only ever operates on batch_size sizes.
#[derive(PartialEq, Debug)]
struct ThreadDistributionPlan {
    pub batch_size: usize,

    /// Number of batches which are processed in the thread pool
    pub thread_pool_batches: usize,

    /// Number of signature checks which are processed in the calling thread
    pub calling_thread_checks: usize,
}

impl ThreadDistributionPlan {
    pub fn new(check_set_size: usize, thread_pool_threads: usize, batch_size: usize) -> Self {
        let overflow_size = if batch_size != 0 {
            check_set_size % batch_size
        } else {
            check_set_size
        };
        let num_full_batches = if batch_size != 0 {
            check_set_size / batch_size
        } else {
            0
        };
        let total_threads_to_split_over = thread_pool_threads + 1;

        // Minimal number of full batches each thread (including the calling thread) works on
        let num_base_batches_each = num_full_batches / total_threads_to_split_over;

        // Number of full batches which will be in a queue (not immediately handled by the calling thread or the thread pool).
        let num_full_overflow_batches = num_full_batches % total_threads_to_split_over;
        let mut calling_thread_checks = (num_base_batches_each * batch_size) + overflow_size;
        let mut thread_pool_batches = num_base_batches_each * thread_pool_threads;
        if num_full_overflow_batches > 0 {
            if overflow_size == 0 {
                // Give the calling thread priority over any batches when there is no excess remainder.
                calling_thread_checks += batch_size;
                thread_pool_batches += num_full_overflow_batches - 1;
            } else {
                thread_pool_batches += num_full_overflow_batches;
            }
        }

        assert!(check_set_size == (thread_pool_batches * batch_size + calling_thread_checks));

        ThreadDistributionPlan {
            thread_pool_batches,
            calling_thread_checks,
            batch_size,
        }
    }

    /// Number of signature checks which are processed in the thread pool
    pub fn thread_pool_checks(&self) -> usize {
        self.thread_pool_batches * self.batch_size
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod thread_distribution_plan {
        use super::*;

        #[test]
        fn all_zero() {
            assert_eq!(
                ThreadDistributionPlan::new(0, 0, 0),
                ThreadDistributionPlan {
                    batch_size: 0,
                    thread_pool_batches: 0,
                    calling_thread_checks: 0
                }
            )
        }

        #[test]
        fn one_calling_thread() {
            assert_eq!(
                ThreadDistributionPlan::new(1, 0, 0),
                ThreadDistributionPlan {
                    batch_size: 0,
                    thread_pool_batches: 0,
                    calling_thread_checks: 1
                }
            )
        }

        #[test]
        fn all_in_calling_thread() {
            assert_eq!(
                ThreadDistributionPlan::new(7, 2, 100),
                ThreadDistributionPlan {
                    batch_size: 100,
                    thread_pool_batches: 0,
                    calling_thread_checks: 7
                }
            )
        }

        #[test]
        fn exactly_one_batch() {
            assert_eq!(
                ThreadDistributionPlan::new(100, 2, 100),
                ThreadDistributionPlan {
                    batch_size: 100,
                    thread_pool_batches: 0,
                    calling_thread_checks: 100
                }
            )
        }

        #[test]
        fn one_above_batch_size() {
            assert_eq!(
                ThreadDistributionPlan::new(101, 2, 100),
                ThreadDistributionPlan {
                    batch_size: 100,
                    thread_pool_batches: 1,
                    calling_thread_checks: 1
                }
            )
        }

        #[test]
        fn two_batches() {
            assert_eq!(
                ThreadDistributionPlan::new(200, 2, 100),
                ThreadDistributionPlan {
                    batch_size: 100,
                    thread_pool_batches: 1,
                    calling_thread_checks: 100
                }
            )
        }

        #[test]
        fn multiple_batches_in_calling_thread() {
            assert_eq!(
                ThreadDistributionPlan::new(400, 2, 100),
                ThreadDistributionPlan {
                    batch_size: 100,
                    thread_pool_batches: 2,
                    calling_thread_checks: 200
                }
            )
        }

        #[test]
        fn no_thread_pool() {
            assert_eq!(
                ThreadDistributionPlan::new(400, 0, 100),
                ThreadDistributionPlan {
                    batch_size: 100,
                    thread_pool_batches: 0,
                    calling_thread_checks: 400
                }
            )
        }
    }

    mod signature_checker {
        use super::*;
        use crate::{Amount, Block, BlockHash, KeyPair, Link, StateBlock};

        // original test: signature_checker.empty
        #[test]
        fn empty() {
            let checker = SignatureChecker::new(0);
            let mut check = SignatureCheckSet {
                messages: Vec::new(),
                pub_keys: Vec::new(),
                signatures: Vec::new(),
                verifications: Vec::new(),
            };
            checker.verify(&mut check);
        }

        // original test: signature_checker.one
        #[test]
        fn one() {
            let checker = SignatureChecker::new(0);
            let key = KeyPair::new();

            let verify_block = |block: &StateBlock, result: i32| {
                let mut check = SignatureCheckSet {
                    messages: vec![block.hash().as_bytes().to_vec()],
                    pub_keys: vec![block.hashables.account.public_key],
                    signatures: vec![block.signature.clone()],
                    verifications: vec![-1],
                };
                checker.verify(&mut check);
                assert_eq!(check.verifications[0], result);
            };

            let mut block = StateBlock::new(
                key.public_key().into(),
                BlockHash::new(),
                key.public_key().into(),
                Amount::zero(),
                Link::new(),
                &key.private_key(),
                &key.public_key(),
                0,
            )
            .unwrap();

            // Check for success for a valid block
            verify_block(&block, 1);

            // Make signaure invalid and check result is incorrect
            block.signature.make_invalid();
            verify_block(&block, 0);
        }
    }
}
