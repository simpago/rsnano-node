use std::{
    collections::VecDeque,
    sync::{Arc, Condvar, Mutex, RwLock},
    thread::JoinHandle,
    time::Duration,
};

use crate::{
    logger_mt::NullLogger, Account, BlockEnum, BlockHash, Epochs, Logger, PublicKey, Signature,
    SignatureCheckSet, SignatureChecker, SignatureVerification,
};

#[derive(Default)]
pub(crate) struct Builder {
    signature_checker: Option<Arc<SignatureChecker>>,
    epochs: Option<Arc<Epochs>>,
    logger: Option<Arc<dyn Logger>>,
    verification_size: Option<usize>,
    enable_timing_logging: bool,
}

impl Builder {
    pub(crate) fn new() -> Self {
        Default::default()
    }

    pub(crate) fn signature_checker(mut self, checker: Arc<SignatureChecker>) -> Self {
        self.signature_checker = Some(checker);
        self
    }

    pub(crate) fn epochs(mut self, epochs: Arc<Epochs>) -> Self {
        self.epochs = Some(epochs);
        self
    }

    pub(crate) fn logger(mut self, logger: Arc<dyn Logger>) -> Self {
        self.logger = Some(logger);
        self
    }

    pub(crate) fn verification_size(mut self, size: usize) -> Self {
        self.verification_size = Some(size);
        self
    }

    pub(crate) fn enable_timing_logging(mut self, enable: bool) -> Self {
        self.enable_timing_logging = enable;
        self
    }

    pub(crate) fn spawn(self) -> std::io::Result<StateBlockSignatureVerification> {
        let thread = Arc::new(StateBlockSignatureVerificationThread {
            condition: Condvar::new(),
            verification_size: self.verification_size.unwrap_or(0),
            signature_checker: self
                .signature_checker
                .unwrap_or_else(|| Arc::new(SignatureChecker::new(0))),
            epochs: self.epochs.unwrap_or_else(|| Arc::new(Epochs::new())),
            logger: self.logger.unwrap_or_else(|| Arc::new(NullLogger::new())),
            timing_logging: self.enable_timing_logging,
            mutable: Mutex::new(ThreadMutableData {
                state_blocks: VecDeque::new(),
                active: false,
                stopped: false,
            }),
            callbacks: Mutex::new(Callbacks {
                blocks_verified_callback: None,
                transition_inactive_callback: None,
            }),
        });

        let thread_clone = thread.clone();
        let join_handle = std::thread::Builder::new()
            .name("State block sig".to_string())
            .spawn(move || {
                thread_clone.run();
            })?;

        Ok(StateBlockSignatureVerification {
            join_handle: Some(join_handle),
            thread,
        })
    }
}

pub(crate) struct StateBlockSignatureVerificationValue {
    pub block: Arc<RwLock<BlockEnum>>,
    pub account: Account,
    pub verification: SignatureVerification,
}

pub(crate) struct StateBlockSignatureVerificationResult {
    pub hashes: Vec<BlockHash>,
    pub signatures: Vec<Signature>,
    pub verifications: Vec<i32>,
    pub items: VecDeque<StateBlockSignatureVerificationValue>,
}

pub(crate) struct StateBlockSignatureVerification {
    join_handle: Option<JoinHandle<()>>,
    thread: Arc<StateBlockSignatureVerificationThread>,
}

impl StateBlockSignatureVerification {
    pub fn builder() -> Builder {
        Builder::new()
    }

    pub(crate) fn set_blocks_verified_callback(
        &self,
        callback: Box<dyn Fn(StateBlockSignatureVerificationResult) + Send + Sync>,
    ) {
        let mut lk = self.thread.callbacks.lock().unwrap();
        lk.blocks_verified_callback = Some(callback);
    }

    pub(crate) fn set_transition_inactive_callback(&self, callback: Box<dyn Fn() + Send + Sync>) {
        let mut lk = self.thread.callbacks.lock().unwrap();
        lk.transition_inactive_callback = Some(callback);
    }

    pub(crate) fn stop(&mut self) -> std::thread::Result<()> {
        {
            let mut lk = self.thread.mutable.lock().unwrap();
            lk.stopped = true;
        }

        if let Some(handle) = self.join_handle.take() {
            self.thread.condition.notify_one();
            handle.join()?;
        }
        Ok(())
    }

    pub(crate) fn add(&self, block: StateBlockSignatureVerificationValue) {
        {
            let mut lk = self.thread.mutable.lock().unwrap();
            lk.state_blocks.push_back(block);
        }
        self.thread.condition.notify_one();
    }

    pub(crate) fn size(&self) -> usize {
        let lk = self.thread.mutable.lock().unwrap();
        lk.state_blocks.len()
    }

    pub(crate) fn is_active(&self) -> bool {
        let lk = self.thread.mutable.lock().unwrap();
        lk.active
    }
}

impl Drop for StateBlockSignatureVerification {
    fn drop(&mut self) {
        self.stop()
            .expect("Could not stop state block verification thread");
    }
}

struct StateBlockSignatureVerificationThread {
    condition: Condvar,
    verification_size: usize,
    signature_checker: Arc<SignatureChecker>,
    epochs: Arc<Epochs>,
    logger: Arc<dyn Logger>,
    mutable: Mutex<ThreadMutableData>,
    callbacks: Mutex<Callbacks>,
    timing_logging: bool,
}

struct ThreadMutableData {
    state_blocks: VecDeque<StateBlockSignatureVerificationValue>,
    active: bool,
    stopped: bool,
}

struct Callbacks {
    blocks_verified_callback:
        Option<Box<dyn Fn(StateBlockSignatureVerificationResult) + Send + Sync>>,
    transition_inactive_callback: Option<Box<dyn Fn() + Send + Sync>>,
}

impl StateBlockSignatureVerificationThread {
    fn run(&self) {
        let mut lk = self.mutable.lock().unwrap();
        while !lk.stopped {
            if !lk.state_blocks.is_empty() {
                let max_verification_batch = if self.verification_size != 0 {
                    self.verification_size
                } else {
                    self.signature_checker.max_size()
                };
                lk.active = true;
                while !lk.state_blocks.is_empty() && !lk.stopped {
                    let items = Self::setup_items(&mut lk, max_verification_batch);
                    drop(lk);
                    self.verify_state_blocks(items);
                    lk = self.mutable.lock().unwrap();
                }
                lk.active = false;
                drop(lk);
                {
                    let callback_lk = self.callbacks.lock().unwrap();
                    if let Some(cb) = &callback_lk.transition_inactive_callback {
                        (cb)();
                    }
                }
                lk = self.mutable.lock().unwrap();
            } else {
                lk = self.condition.wait(lk).unwrap();
            }
        }
    }

    fn setup_items(
        data: &mut ThreadMutableData,
        max_count: usize,
    ) -> VecDeque<StateBlockSignatureVerificationValue> {
        let mut items = VecDeque::new();
        if data.state_blocks.len() <= max_count {
            std::mem::swap(&mut items, &mut data.state_blocks);
        } else {
            for _ in 0..max_count {
                if let Some(item) = data.state_blocks.pop_front() {
                    items.push_back(item);
                }
            }
            debug_assert!(!data.state_blocks.is_empty());
        }

        items
    }

    fn verify_state_blocks(&self, items: VecDeque<StateBlockSignatureVerificationValue>) {
        if items.is_empty() {
            return;
        }

        let now = std::time::Instant::now();
        let size = items.len();
        let mut hashes: Vec<BlockHash> = Vec::with_capacity(size);
        let mut messages: Vec<Vec<u8>> = Vec::with_capacity(size);
        let mut accounts: Vec<Account> = Vec::with_capacity(size);
        let mut pub_keys: Vec<PublicKey> = Vec::with_capacity(size);
        let mut block_signatures: Vec<Signature> = Vec::with_capacity(size);
        let verifications = vec![0; size];

        for i in &items {
            let guard = i.block.read().unwrap();
            let block = guard.as_block();
            hashes.push(block.hash());
            messages.push(block.hash().to_bytes().to_vec());
            let mut account_l = *block.account();
            if !block.link().is_zero() && self.epochs.is_epoch_link(&block.link()) {
                account_l = self
                    .epochs
                    .signer(self.epochs.epoch(&block.link()).unwrap())
                    .unwrap()
                    .into();
            } else if !i.account.is_zero() {
                account_l = i.account;
            }
            accounts.push(account_l);
            pub_keys.push(account_l.public_key);
            block_signatures.push(block.block_signature().clone())
        }

        let mut check = SignatureCheckSet {
            messages,
            pub_keys,
            signatures: block_signatures,
            verifications,
        };
        self.signature_checker.verify(&mut check);

        if self.timing_logging && now.elapsed() > Duration::from_millis(10) {
            self.logger.try_log(&format!(
                "Batch verified {} state blocks in {} ms",
                size,
                now.elapsed().as_millis()
            ));
        }

        let result = StateBlockSignatureVerificationResult {
            hashes,
            signatures: check.signatures,
            verifications: check.verifications,
            items,
        };

        let lk = self.callbacks.lock().unwrap();
        if let Some(cb) = &lk.blocks_verified_callback {
            (cb)(result);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{KeyPair, StateBlockBuilder};

    #[test]
    fn verify_one_block() {
        let verification = StateBlockSignatureVerification::builder().spawn().unwrap();
        let verified_pair: Arc<(
            Mutex<Option<StateBlockSignatureVerificationResult>>,
            Condvar,
        )> = Arc::new((Mutex::new(None), Condvar::new()));
        let verified_pair2 = Arc::clone(&verified_pair);

        verification.set_blocks_verified_callback(Box::new(move |result| {
            let (lock, cvar) = &*verified_pair2;
            let mut verified = lock.lock().unwrap();
            *verified = Some(result);
            cvar.notify_one();
        }));

        let inactive_pair = Arc::new((Mutex::new(false), Condvar::new()));
        let inactive_pair2 = Arc::clone(&inactive_pair);

        verification.set_transition_inactive_callback(Box::new(move || {
            let (lock, cvar) = &*inactive_pair2;
            let mut inactive = lock.lock().unwrap();
            *inactive = true;
            cvar.notify_one();
        }));

        let keys = KeyPair::new();
        let account = keys.public_key().into();
        let block = StateBlockBuilder::new()
            .account(account)
            .sign(&keys)
            .build()
            .unwrap();
        let block = Arc::new(RwLock::new(BlockEnum::State(block)));

        verification.add(StateBlockSignatureVerificationValue {
            block,
            account,
            verification: SignatureVerification::Unknown,
        });

        let (lock, cvar) = &*verified_pair;
        let mut verified = lock.lock().unwrap();
        while verified.is_none() {
            verified = cvar.wait(verified).unwrap();
        }
        let result = verified.as_ref().unwrap();
        assert_eq!(result.verifications.len(), 1);
        assert_eq!(result.verifications[0], 1);

        let (lock, cvar) = &*inactive_pair;
        let mut inactive = lock.lock().unwrap();
        while !*inactive {
            inactive = cvar.wait(inactive).unwrap();
        }
        assert_eq!(*inactive, true);
    }
}
