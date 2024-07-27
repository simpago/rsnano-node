use std::time::Duration;

#[derive(Clone)]
pub struct HintedSchedulerConfig {
    pub enabled: bool,
    pub check_interval: Duration,
    pub block_cooldown: Duration,
    pub hinting_theshold_percent: u32,
    pub vacancy_threshold_percent: u32,
}

impl HintedSchedulerConfig {
    pub fn default_for_dev_network() -> Self {
        Self {
            check_interval: Duration::from_millis(100),
            block_cooldown: Duration::from_millis(100),
            ..Default::default()
        }
    }
}

impl Default for HintedSchedulerConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            check_interval: Duration::from_millis(1000),
            block_cooldown: Duration::from_millis(5000),
            hinting_theshold_percent: 10,
            vacancy_threshold_percent: 20,
        }
    }
}
