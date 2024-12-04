use super::Wallet;
use rsnano_core::Amount;
use std::{
    collections::BTreeMap,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Condvar, Mutex, MutexGuard,
    },
    thread::JoinHandle,
};

pub struct WalletActionThread {
    action_loop: Arc<WalletActionLoop>,
    join_handle: Mutex<Option<JoinHandle<()>>>,
}

impl Drop for WalletActionThread {
    fn drop(&mut self) {
        assert!(
            self.join_handle.lock().unwrap().is_none(),
            "wallet action thread wasn't stopped"
        );
    }
}

impl WalletActionThread {
    pub fn new() -> Self {
        Self {
            action_loop: Arc::new(WalletActionLoop::new()),
            join_handle: Mutex::new(None),
        }
    }

    pub fn start(&self) {
        let loop_clone = Arc::clone(&self.action_loop);
        let mut guard = self.join_handle.lock().unwrap();
        assert!(guard.is_none(), "wallet action thread already running");
        *guard = Some(
            std::thread::Builder::new()
                .name("Wallet actions".to_string())
                .spawn(move || {
                    loop_clone.do_wallet_actions();
                })
                .unwrap(),
        );
    }

    pub fn stop(&self) {
        self.action_loop.stop();
        let join_handle = self.join_handle.lock().unwrap().take();
        if let Some(join_handle) = join_handle {
            join_handle.join().unwrap();
        }
    }

    pub fn queue_wallet_action(
        &self,
        amount: Amount,
        wallet: Arc<Wallet>,
        action: Box<dyn Fn(Arc<Wallet>) + Send>,
    ) {
        self.action_loop.queue_wallet_action(amount, wallet, action);
    }

    pub fn len(&self) -> usize {
        self.action_loop.len()
    }

    pub fn set_observer(&self, observer: Box<dyn Fn(bool) + Send>) {
        self.action_loop.set_observer(observer);
    }

    pub fn lock_safe(
        &self,
    ) -> MutexGuard<BTreeMap<Amount, Vec<(Arc<Wallet>, Box<dyn Fn(Arc<Wallet>) + Send>)>>> {
        self.action_loop.mutex.lock().unwrap()
    }

    pub unsafe fn lock(
        &self,
    ) -> MutexGuard<'static, BTreeMap<Amount, Vec<(Arc<Wallet>, Box<dyn Fn(Arc<Wallet>) + Send>)>>>
    {
        let guard = self.action_loop.mutex.lock().unwrap();
        std::mem::transmute::<
            MutexGuard<BTreeMap<Amount, Vec<(Arc<Wallet>, Box<dyn Fn(Arc<Wallet>) + Send>)>>>,
            MutexGuard<
                'static,
                BTreeMap<Amount, Vec<(Arc<Wallet>, Box<dyn Fn(Arc<Wallet>) + Send>)>>,
            >,
        >(guard)
    }
}

struct WalletActionLoop {
    mutex: Mutex<BTreeMap<Amount, Vec<(Arc<Wallet>, Box<dyn Fn(Arc<Wallet>) + Send>)>>>,
    stopped: AtomicBool,
    condition: Condvar,
    observer: Mutex<Box<dyn Fn(bool) + Send>>,
}

impl WalletActionLoop {
    fn new() -> Self {
        Self {
            mutex: Mutex::new(BTreeMap::new()),
            stopped: AtomicBool::new(false),
            condition: Condvar::new(),
            observer: Mutex::new(Box::new(|_| {})),
        }
    }

    fn stop(&self) {
        {
            let mut guard = self.mutex.lock().unwrap();
            self.stopped.store(true, Ordering::SeqCst);
            guard.clear();
        }
        self.condition.notify_all();
    }

    fn queue_wallet_action(
        &self,
        amount: Amount,
        wallet: Arc<Wallet>,
        action: Box<dyn Fn(Arc<Wallet>) + Send>,
    ) {
        {
            let mut guard = self.mutex.lock().unwrap();
            guard.entry(amount).or_default().push((wallet, action));
        }
        self.condition.notify_all();
    }

    fn len(&self) -> usize {
        self.mutex.lock().unwrap().len()
    }

    fn set_observer(&self, observer: Box<dyn Fn(bool) + Send>) {
        *self.observer.lock().unwrap() = observer;
    }

    fn do_wallet_actions(&self) {
        let mut guard = self.mutex.lock().unwrap();
        while !self.stopped.load(Ordering::SeqCst) {
            if let Some((_, wallets)) = guard.pop_first() {
                for (wallet, action) in wallets {
                    if self.stopped.load(Ordering::SeqCst) {
                        break;
                    }

                    if wallet.live() {
                        drop(guard);
                        (self.observer.lock().unwrap())(true);
                        action(wallet);
                        (self.observer.lock().unwrap())(false);
                        guard = self.mutex.lock().unwrap();
                    }
                }
            } else {
                guard = self.condition.wait(guard).unwrap();
            }
        }
    }
}
