use std::{
    ops::Deref,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Mutex,
    },
};

pub(crate) struct NullableRuntime {
    strategy: RuntimeStrategy,
    blocking_spawns: AtomicUsize,
}

#[allow(dead_code)]
impl NullableRuntime {
    pub(crate) fn new(handle: tokio::runtime::Handle) -> Self {
        Self::with_strategy(RuntimeStrategy::Real(RealRuntime(handle)))
    }

    pub(crate) fn new_null() -> Self {
        Self::with_strategy(RuntimeStrategy::Null(StubRuntime(Mutex::new(None))))
    }

    fn with_strategy(strategy: RuntimeStrategy) -> Self {
        Self {
            strategy,
            blocking_spawns: AtomicUsize::new(0),
        }
    }

    pub(crate) fn blocking_spawns(&self) -> usize {
        self.blocking_spawns.load(Ordering::SeqCst)
    }

    pub(crate) fn spawn_blocking(&self, f: impl FnOnce() + Send + Sync + 'static) {
        self.blocking_spawns.fetch_add(1, Ordering::SeqCst);
        self.strategy.spawn_blocking(Box::new(f));
    }

    pub(crate) fn run_nulled_blocking_task(&self) {
        self.strategy.run_nulled_blocking_task();
    }
}

impl Default for NullableRuntime {
    fn default() -> Self {
        Self::new(tokio::runtime::Handle::current())
    }
}

#[allow(dead_code)]
enum RuntimeStrategy {
    Real(RealRuntime),
    Null(StubRuntime),
}

impl Deref for RuntimeStrategy {
    type Target = dyn RuntimeImpl;

    fn deref(&self) -> &Self::Target {
        match self {
            RuntimeStrategy::Real(i) => i,
            RuntimeStrategy::Null(i) => i,
        }
    }
}

#[allow(dead_code)]
trait RuntimeImpl {
    fn spawn_blocking(&self, f: Box<dyn FnOnce() + Send + Sync>);
    fn run_nulled_blocking_task(&self);
}

struct RealRuntime(tokio::runtime::Handle);

impl RuntimeImpl for RealRuntime {
    fn spawn_blocking(&self, f: Box<dyn FnOnce() + Send + Sync>) {
        self.0.spawn_blocking(f);
    }

    fn run_nulled_blocking_task(&self) {
        panic!("run_nulled_blocking_task must not be called on a real runtime!")
    }
}

struct StubRuntime(Mutex<Option<Box<dyn FnOnce() + Send + Sync>>>);

impl RuntimeImpl for StubRuntime {
    fn spawn_blocking(&self, f: Box<dyn FnOnce() + Send + Sync>) {
        *self.0.lock().unwrap() = Some(f);
    }

    fn run_nulled_blocking_task(&self) {
        if let Some(f) = self.0.lock().unwrap().take() {
            f();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        sync::{
            atomic::{AtomicUsize, Ordering},
            Arc,
        },
        time::{Duration, Instant},
    };

    mod real {
        use super::*;

        #[tokio::test]
        async fn initial_state() {
            let runtime = NullableRuntime::default();
            assert_eq!(runtime.blocking_spawns(), 0);
        }

        #[tokio::test]
        async fn spawn_blocking() {
            let runtime = NullableRuntime::default();
            let called = Arc::new(AtomicUsize::new(0));
            let called2 = called.clone();
            runtime.spawn_blocking(move || {
                called2.fetch_add(1, Ordering::SeqCst);
            });
            assert_eq!(runtime.blocking_spawns(), 1);

            let start = Instant::now();
            loop {
                if called.load(Ordering::SeqCst) > 0 {
                    break;
                }
                if start.elapsed() > Duration::from_secs(5) {
                    break;
                }
                std::thread::yield_now();
            }
            assert_eq!(called.load(Ordering::SeqCst), 1);
        }

        #[tokio::test]
        #[should_panic(expected = "run_nulled_blocking_task must not be called on a real runtime")]
        async fn cannot_trigger_blocking_task() {
            let runtime = NullableRuntime::default();
            runtime.spawn_blocking(|| {});
            runtime.run_nulled_blocking_task();
        }
    }

    mod nullability {
        use super::*;

        #[test]
        fn spawn_blocking() {
            let runtime = NullableRuntime::new_null();
            let called = Arc::new(AtomicUsize::new(0));
            let called2 = called.clone();
            runtime.spawn_blocking(move || {
                called2.fetch_add(1, Ordering::SeqCst);
            });
            assert_eq!(runtime.blocking_spawns(), 1, "spawn count");
            assert_eq!(called.load(Ordering::SeqCst), 0, "called count");

            runtime.run_nulled_blocking_task();
            assert_eq!(called.load(Ordering::SeqCst), 1, "called count");
        }
    }
}
