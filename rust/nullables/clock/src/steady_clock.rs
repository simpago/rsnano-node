use std::{
    ops::{Add, Sub},
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
    Stub(u64),
}

impl TimeSource {
    fn now(&self) -> u64 {
        match self {
            TimeSource::System(instant) => instant.elapsed().as_millis() as u64,
            TimeSource::Stub(value) => *value,
        }
    }
}

const DEFAULT_STUB_DURATION: u64 = 1000 * 60 * 60 * 24 * 365;

#[derive(PartialEq, Eq, PartialOrd, Ord, Debug, Clone, Copy, Default)]
pub struct Timestamp(u64);

impl Timestamp {
    pub const MAX: Self = Self(u64::MAX);

    pub const fn new_test_instance() -> Self {
        Self(DEFAULT_STUB_DURATION)
    }

    pub fn elapsed(&self, now: Timestamp) -> Duration {
        Duration::from_millis(now.0.checked_sub(self.0).unwrap_or_default())
    }

    pub fn checked_sub(&self, rhs: Duration) -> Option<Self> {
        self.0.checked_sub(rhs.as_millis() as u64).map(|i| Self(i))
    }
}

impl Add<Duration> for Timestamp {
    type Output = Timestamp;

    fn add(self, rhs: Duration) -> Self::Output {
        Self(self.0.add(rhs.as_millis() as u64))
    }
}

impl Sub<Timestamp> for Timestamp {
    type Output = Duration;

    fn sub(self, rhs: Timestamp) -> Self::Output {
        Duration::from_millis(self.0 - rhs.0)
    }
}

impl Sub<Duration> for Timestamp {
    type Output = Timestamp;

    fn sub(self, rhs: Duration) -> Self::Output {
        Self(self.0 - rhs.as_millis() as u64)
    }
}

impl From<u64> for Timestamp {
    fn from(value: u64) -> Self {
        Self(value)
    }
}

impl From<Timestamp> for u64 {
    fn from(value: Timestamp) -> Self {
        value.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;

    mod timestamp {
        use super::*;

        #[test]
        fn add_duration() {
            assert_eq!(
                Timestamp::from(1000) + Duration::from_millis(300),
                Timestamp::from(1300)
            );
        }

        #[test]
        fn sub() {
            assert_eq!(
                Timestamp::from(1000) - Timestamp::from(300),
                Duration::from_millis(700)
            );
        }
    }

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
