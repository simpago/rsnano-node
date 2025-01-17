use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Condvar, Mutex,
    },
    thread::JoinHandle,
    time::Duration,
};

use rsnano_output_tracker::{OutputListenerMt, OutputTrackerMt};

// Runs a task periodically in it's own thread
pub struct TimerThread<T: Runnable + 'static> {
    thread_name: String,
    task: Mutex<Option<T>>,
    thread: Mutex<Option<JoinHandle<()>>>,
    cancel_token: CancellationToken,
    start_listener: OutputListenerMt<TimerStartEvent>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TimerStartEvent {
    pub thread_name: String,
    pub start_type: TimerStartType,
    pub interval: Duration,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum TimerStartType {
    Start,
    StartDelayed,
    RunOnceThenStart,
}

impl<T: Runnable> TimerThread<T> {
    pub fn new(name: impl Into<String>, task: T) -> Self {
        Self {
            thread_name: name.into(),
            task: Mutex::new(Some(task)),
            thread: Mutex::new(None),
            cancel_token: CancellationToken::new(),
            start_listener: OutputListenerMt::new(),
        }
    }

    pub fn is_running(&self) -> bool {
        self.thread.lock().unwrap().is_some()
    }

    pub fn track_start(&self) -> Arc<OutputTrackerMt<TimerStartEvent>> {
        self.start_listener.track()
    }

    /// Start the thread which periodically runs the task
    pub fn start(&self, interval: Duration) {
        self.start_impl(interval, TimerStartType::Start);
    }

    /// Starts the thread and waits for the given interval before the first run
    pub fn start_delayed(&self, interval: Duration) {
        self.start_impl(interval, TimerStartType::StartDelayed);
    }

    /// Runs the task in the current thread once before the thread is started
    pub fn run_once_then_start(&self, interval: Duration) {
        self.start_impl(interval, TimerStartType::RunOnceThenStart);
    }

    fn start_impl(&self, interval: Duration, start_type: TimerStartType) {
        self.start_listener.emit(TimerStartEvent {
            thread_name: self.thread_name.clone(),
            interval,
            start_type,
        });

        let mut task = self
            .task
            .lock()
            .unwrap()
            .take()
            .expect("task already taken");

        let cancel_token = self.cancel_token.clone();

        if start_type == TimerStartType::RunOnceThenStart {
            task.run(&cancel_token);
        }

        let handle = std::thread::Builder::new()
            .name(self.thread_name.clone())
            .spawn(move || {
                if start_type == TimerStartType::Start {
                    task.run(&cancel_token);
                }

                while !cancel_token.wait_for_cancellation(interval) {
                    task.run(&cancel_token);
                }
            })
            .unwrap();

        *self.thread.lock().unwrap() = Some(handle);
    }

    pub fn stop(&self) {
        self.cancel_token.cancel();
        let handle = self.thread.lock().unwrap().take();
        if let Some(handle) = handle {
            handle.join().unwrap();
        }
    }
}

impl<T: Runnable> Drop for TimerThread<T> {
    fn drop(&mut self) {
        self.stop();
    }
}

pub trait Runnable: Send {
    fn run(&mut self, cancel_token: &CancellationToken);
}

#[derive(Clone)]
pub struct CancellationToken {
    strategy: Arc<CancellationTokenStrategy>,
    wait_listener: Arc<OutputListenerMt<Duration>>,
}

impl CancellationToken {
    pub fn new() -> Self {
        Self {
            strategy: Arc::new(CancellationTokenStrategy::Real(CancellationTokenImpl {
                mutex: Mutex::new(()),
                condition: Condvar::new(),
                stopped: AtomicBool::new(false),
            })),
            wait_listener: Arc::new(OutputListenerMt::new()),
        }
    }

    pub fn new_null() -> Self {
        Self::new_null_with_uncancelled_waits(usize::MAX)
    }

    pub fn new_null_with_uncancelled_waits(uncancelled_wait_count: usize) -> Self {
        Self {
            strategy: Arc::new(CancellationTokenStrategy::Nulled(
                CancellationTokenStub::new(uncancelled_wait_count),
            )),
            wait_listener: Arc::new(OutputListenerMt::new()),
        }
    }

    pub fn wait_for_cancellation(&self, timeout: Duration) -> bool {
        self.wait_listener.emit(timeout);
        match &*self.strategy {
            CancellationTokenStrategy::Real(i) => i.wait_for_cancellation(timeout),
            CancellationTokenStrategy::Nulled(i) => i.wait_for_cancellation(),
        }
    }

    pub fn cancel(&self) {
        match &*self.strategy {
            CancellationTokenStrategy::Real(i) => i.cancel(),
            CancellationTokenStrategy::Nulled(_) => {}
        }
    }

    pub fn is_cancelled(&self) -> bool {
        match &*self.strategy {
            CancellationTokenStrategy::Real(i) => i.is_cancelled(),
            CancellationTokenStrategy::Nulled(i) => i.is_cancelled(),
        }
    }

    pub fn track_waits(&self) -> Arc<OutputTrackerMt<Duration>> {
        self.wait_listener.track()
    }
}

enum CancellationTokenStrategy {
    Real(CancellationTokenImpl),
    Nulled(CancellationTokenStub),
}

struct CancellationTokenImpl {
    mutex: Mutex<()>,
    condition: Condvar,
    stopped: AtomicBool,
}

impl CancellationTokenImpl {
    fn wait_for_cancellation(&self, timeout: Duration) -> bool {
        let guard = self.mutex.lock().unwrap();
        if self.is_cancelled() {
            return true;
        }

        drop(
            self.condition
                .wait_timeout_while(guard, timeout, |_| !self.is_cancelled())
                .unwrap()
                .0,
        );

        self.is_cancelled()
    }

    fn cancel(&self) {
        {
            let _guard = self.mutex.lock().unwrap();
            self.stopped.store(true, Ordering::SeqCst);
        }
        self.condition.notify_all();
    }

    fn is_cancelled(&self) -> bool {
        self.stopped.load(Ordering::SeqCst)
    }
}

struct CancellationTokenStub {
    uncancelled_waits: Mutex<usize>,
    cancelled: AtomicBool,
}

impl CancellationTokenStub {
    fn new(uncancelled_waits: usize) -> Self {
        Self {
            cancelled: AtomicBool::new(uncancelled_waits == 0),
            uncancelled_waits: Mutex::new(uncancelled_waits),
        }
    }

    fn wait_for_cancellation(&self) -> bool {
        let mut waits = self.uncancelled_waits.lock().unwrap();
        if *waits > 0 {
            *waits -= 1;
            false
        } else {
            self.cancelled.store(true, Ordering::SeqCst);
            true
        }
    }

    fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_be_nulled() {
        let token = CancellationToken::new_null();
        assert_eq!(token.wait_for_cancellation(Duration::MAX), false);
        assert_eq!(token.is_cancelled(), false);
        assert_eq!(token.wait_for_cancellation(Duration::MAX), false);
        assert_eq!(token.is_cancelled(), false);
    }

    #[test]
    fn nulled_cancellation_token_returns_configured_responses() {
        let token = CancellationToken::new_null_with_uncancelled_waits(2);

        assert_eq!(token.wait_for_cancellation(Duration::MAX), false);
        assert_eq!(token.is_cancelled(), false);
        assert_eq!(token.wait_for_cancellation(Duration::MAX), false);
        assert_eq!(token.is_cancelled(), false);
        assert_eq!(token.wait_for_cancellation(Duration::MAX), true);
        assert_eq!(token.is_cancelled(), true);
        assert_eq!(token.wait_for_cancellation(Duration::MAX), true);
        assert_eq!(token.is_cancelled(), true);
    }

    #[test]
    fn can_track_waits() {
        let token = CancellationToken::new_null();
        let wait_tracker = token.track_waits();
        let duration = Duration::from_secs(123);

        token.wait_for_cancellation(duration);

        assert_eq!(wait_tracker.output(), [duration]);
    }
}
