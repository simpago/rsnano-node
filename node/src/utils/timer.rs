#[cfg(feature = "output_tracking")]
use std::sync::{Arc, Mutex};

#[cfg(feature = "output_tracking")]
use rsnano_output_tracker::{OutputListenerMt, OutputTrackerMt};

pub trait TimerStrategy: Send {
    fn schedule_with_delay<F>(&self, delay: chrono::Duration, cb: F)
    where
        F: 'static + FnMut() + Send;
}

pub struct TimerWrapper(timer::Timer);

impl TimerStrategy for TimerWrapper {
    fn schedule_with_delay<F>(&self, delay: chrono::Duration, cb: F)
    where
        F: 'static + FnMut() + Send,
    {
        self.0.schedule_with_delay(delay, cb).ignore();
    }
}

pub struct NullTimer;
impl TimerStrategy for NullTimer {
    fn schedule_with_delay<F>(&self, _delay: chrono::Duration, _cb: F)
    where
        F: 'static + FnMut() + Send,
    {
    }
}

pub struct Timer<T: TimerStrategy = TimerWrapper> {
    timer: T,
    #[cfg(feature = "output_tracking")]
    listener: OutputListenerMt<TimerEvent>,
}

#[cfg(feature = "output_tracking")]
#[derive(Clone)]
pub struct TimerEvent {
    callback: Arc<dyn Fn() + Send + Sync>,
    pub delay: chrono::Duration,
}

#[cfg(feature = "output_tracking")]
impl TimerEvent {
    pub fn execute_callback(&self) {
        (self.callback)()
    }
}

impl Timer<NullTimer> {
    pub fn new_null() -> Self {
        Self::new_with(NullTimer {})
    }
}

impl Timer<TimerWrapper> {
    pub fn new() -> Self {
        Self::new_with(TimerWrapper(timer::Timer::new()))
    }
}

impl<T: TimerStrategy> Timer<T> {
    fn new_with(t: T) -> Self {
        Self {
            timer: t,
            #[cfg(feature = "output_tracking")]
            listener: OutputListenerMt::new(),
        }
    }

    #[cfg(feature = "output_tracking")]
    pub fn track(&self) -> Arc<OutputTrackerMt<TimerEvent>> {
        self.listener.track()
    }

    pub fn schedule_with_delay<F>(&self, delay: chrono::Duration, cb: F)
    where
        F: 'static + FnMut() + Send,
    {
        #[cfg(feature = "output_tracking")]
        let cb = {
            let option_cb = Arc::new(Mutex::new(Some(cb)));
            let arc_cb: Arc<dyn Fn() + Send + Sync> = Arc::new(move || {
                let cb = option_cb.lock().unwrap().take();
                if let Some(mut cb) = cb {
                    cb();
                }
            });

            self.listener.emit(TimerEvent {
                callback: arc_cb.clone(),
                delay,
            });
            move || arc_cb()
        };

        self.timer.schedule_with_delay(delay, cb);
    }
}

#[cfg(test)]
mod tests {
    use std::{sync::mpsc::channel, time::Duration};

    use super::*;

    #[test]
    fn schedule_with_delay() {
        let t = Timer::new();
        let (tx, rx) = channel();
        t.schedule_with_delay(chrono::Duration::microseconds(1), move || {
            tx.send("done").unwrap();
        });
        let result = rx.recv_timeout(Duration::from_millis(300));
        assert_eq!(result, Ok("done"));
    }

    #[test]
    fn can_be_tracked() {
        let t = Timer::new_null();
        let tracker = t.track();
        let (tx, rx) = channel();
        t.schedule_with_delay(chrono::Duration::seconds(10), move || {
            tx.send("done").unwrap();
        });
        let output = tracker.output();
        assert_eq!(output.len(), 1, "nothing tracked");
        assert_eq!(output[0].delay, chrono::Duration::seconds(10), "delay");
        output[0].execute_callback();
        let result = rx.recv_timeout(Duration::from_millis(300));
        assert_eq!(result, Ok("done"));
    }
}
