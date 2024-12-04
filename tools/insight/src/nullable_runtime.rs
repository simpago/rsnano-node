use std::{
    future::Future,
    pin::Pin,
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
        Self::with_strategy(RuntimeStrategy::Real(handle))
    }

    pub(crate) fn new_null() -> Self {
        Self::with_strategy(RuntimeStrategy::Null(StubRuntime::new()))
    }

    fn with_strategy(strategy: RuntimeStrategy) -> Self {
        Self {
            strategy,
            blocking_spawns: AtomicUsize::new(0),
        }
    }

    pub(crate) fn spawn<F>(&self, f: F)
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        match &self.strategy {
            RuntimeStrategy::Real(s) => {
                s.spawn(f);
            }
            RuntimeStrategy::Null(s) => {
                s.spawn(f);
            }
        }
    }

    pub(crate) fn blocking_spawns(&self) -> usize {
        self.blocking_spawns.load(Ordering::SeqCst)
    }

    pub(crate) fn spawn_blocking(&self, f: impl FnOnce() + Send + Sync + 'static) {
        self.blocking_spawns.fetch_add(1, Ordering::SeqCst);
        match &self.strategy {
            RuntimeStrategy::Real(s) => {
                s.spawn_blocking(f);
            }
            RuntimeStrategy::Null(s) => s.spawn_blocking(f),
        }
    }

    pub(crate) async fn run_nulled_spawn(&self) {
        match &self.strategy {
            RuntimeStrategy::Real(_) => {
                panic!("run_nulled_spawn must not be called on a real runtime!")
            }
            RuntimeStrategy::Null(s) => s.run_nulled_spawn().await,
        }
    }

    pub(crate) fn run_nulled_blocking_task(&self) {
        match &self.strategy {
            RuntimeStrategy::Real(_) => {
                panic!("run_nulled_blocking_task must not be called on a real runtime!")
            }
            RuntimeStrategy::Null(s) => s.run_nulled_blocking_task(),
        }
    }
}

impl Default for NullableRuntime {
    fn default() -> Self {
        Self::new(tokio::runtime::Handle::current())
    }
}

#[allow(dead_code)]
enum RuntimeStrategy {
    Real(tokio::runtime::Handle),
    Null(StubRuntime),
}

struct StubRuntime {
    blocking: Mutex<Option<Box<dyn FnOnce() + Send + Sync>>>,
    spawns: Mutex<Option<Pin<Box<dyn Future<Output = ()> + Send>>>>,
}

impl StubRuntime {
    fn new() -> Self {
        Self {
            blocking: Mutex::new(None),
            spawns: Mutex::new(None),
        }
    }

    fn spawn_blocking(&self, f: impl FnOnce() + Send + Sync + 'static) {
        *self.blocking.lock().unwrap() = Some(Box::new(f));
    }

    fn run_nulled_blocking_task(&self) {
        if let Some(f) = self.blocking.lock().unwrap().take() {
            f();
        }
    }

    fn spawn<F>(&self, f: F)
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        *self.spawns.lock().unwrap() = Some(Box::pin(async move {
            f.await;
        }));
    }

    async fn run_nulled_spawn(&self) {
        if let Some(f) = self.spawns.lock().unwrap().take() {
            f.await;
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
