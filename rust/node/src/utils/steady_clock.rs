use std::{
    ops::Add,
    time::{Duration, Instant},
};

pub struct SteadyClock {
    time_source: TimeSource,
}

impl SteadyClock {
    pub fn new_null() -> Self {
        Self {
            time_source: TimeSource::Stub(DEFAULT_STUB_DURATION),
        }
    }

    pub fn now(&self) -> Timestamp {
        Timestamp(self.time_source.now())
    }
}

impl Default for SteadyClock {
    fn default() -> Self {
        SteadyClock {
            time_source: TimeSource::System(Instant::now()),
        }
    }
}

enum TimeSource {
    System(Instant),
    Stub(Duration),
}

impl TimeSource {
    fn now(&self) -> Duration {
        match self {
            TimeSource::System(instant) => instant.elapsed(),
            TimeSource::Stub(value) => *value,
        }
    }
}

const DEFAULT_STUB_DURATION: Duration = Duration::from_secs(60 * 60 * 24 * 365);

#[derive(PartialEq, Eq, PartialOrd, Ord, Debug, Clone, Copy, Default)]
pub struct Timestamp(Duration);

impl Timestamp {
    pub const MAX: Self = Self(Duration::MAX);

    pub const fn new_test_instance() -> Self {
        Self(DEFAULT_STUB_DURATION)
    }

    pub fn elapsed(&self, now: Timestamp) -> Duration {
        now.0.checked_sub(self.0).unwrap_or_default()
    }

    pub fn checked_sub(&self, rhs: Duration) -> Option<Self> {
        self.0.checked_sub(rhs).map(|i| Self(i))
    }
}

impl Add<Duration> for Timestamp {
    type Output = Timestamp;

    fn add(self, rhs: Duration) -> Self::Output {
        Self(self.0.add(rhs))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;

    #[test]
    fn now() {
        let clock = SteadyClock::default();
        let now1 = clock.now();
        sleep(Duration::from_millis(1));
        let now2 = clock.now();
        assert!(now2 > now1);
    }

    mod nullability {
        use super::*;
        #[test]
        fn can_be_nulled() {
            let clock = SteadyClock::new_null();
            let now1 = clock.now();
            let now2 = clock.now();
            assert_eq!(now1, now2);
        }
    }
}
