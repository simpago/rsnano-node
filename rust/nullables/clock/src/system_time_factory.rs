use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub struct SystemTimeFactory(TimeStrategy);

impl SystemTimeFactory {
    pub fn new_null() -> Self {
        Self(TimeStrategy::Stub(
            UNIX_EPOCH + Duration::from_secs(60 * 60 * 24 * 365 * 50),
        ))
    }

    pub fn new_null_with(configured_response: SystemTime) -> Self {
        Self(TimeStrategy::Stub(configured_response))
    }

    pub fn now(&self) -> SystemTime {
        match &self.0 {
            TimeStrategy::Real => SystemTime::now(),
            TimeStrategy::Stub(now) => *now,
        }
    }
}

impl Default for SystemTimeFactory {
    fn default() -> Self {
        Self(TimeStrategy::Real)
    }
}

enum TimeStrategy {
    Real,
    Stub(SystemTime),
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, UNIX_EPOCH};

    #[test]
    fn get_real_system_time() {
        let system_now = SystemTime::now();
        let now = SystemTimeFactory::default().now();
        assert!(now >= system_now - Duration::from_secs(60 * 10));
        assert!(now < system_now + Duration::from_secs(60 * 10));
    }

    #[test]
    fn nulled_time_factory_returns_stub_time() {
        assert_eq!(
            SystemTimeFactory::new_null().now(),
            UNIX_EPOCH + Duration::from_secs(60 * 60 * 24 * 365 * 50)
        );
    }

    #[test]
    fn nulled_time_factory_returns_configured_response() {
        let configured_response = UNIX_EPOCH + Duration::from_secs(1_000_000);
        assert_eq!(
            SystemTimeFactory::new_null_with(configured_response).now(),
            configured_response
        );
    }
}
