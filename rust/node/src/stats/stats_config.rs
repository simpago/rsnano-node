use anyhow::Result;
use rsnano_core::utils::TomlWriter;
use std::time::Duration;

#[derive(Clone)]
pub struct StatsConfig {
    /** How many sample intervals to keep in the ring buffer */
    pub max_samples: usize,

    /** How often to log sample array, in milliseconds. Default is 0 (no logging) */
    pub log_samples_interval: Duration,

    /** How often to log counters, in milliseconds. Default is 0 (no logging) */
    pub log_counters_interval: Duration,

    /** Maximum number of log outputs before rotating the file */
    pub log_rotation_count: usize,

    /** If true, write headers on each counter or samples writeout. The header contains log type and the current wall time. */
    pub log_headers: bool,

    /** Filename for the counter log  */
    pub log_counters_filename: String,

    /** Filename for the sampling log */
    pub log_samples_filename: String,
}

impl Default for StatsConfig {
    fn default() -> Self {
        Self {
            max_samples: 1024 * 16,
            log_samples_interval: Duration::ZERO,
            log_counters_interval: Duration::ZERO,
            log_rotation_count: 100,
            log_headers: true,
            log_counters_filename: "counters.stat".to_string(),
            log_samples_filename: "samples.stat".to_string(),
        }
    }
}

impl StatsConfig {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn serialize_toml(&self, toml: &mut dyn TomlWriter) -> Result<()> {
        toml.put_usize(
            "max_samples",
            self.max_samples,
            "Maximum number ofmany samples to keep in the ring buffer.\ntype:uint64",
        )?;

        toml.put_child("log", &mut |log|{
            log.put_bool("headers", self.log_headers, "If true, write headers on each counter or samples writeout.\nThe header contains log type and the current wall time.\ntype:bool")?;
            log.put_usize("interval_counters", self.log_counters_interval.as_millis() as usize, "How often to log counters. 0 disables logging.\ntype:milliseconds")?;
            log.put_usize("interval_samples", self.log_samples_interval.as_millis() as usize, "How often to log samples. 0 disables logging.\ntype:milliseconds")?;
            log.put_usize("rotation_count", self.log_rotation_count, "Maximum number of log outputs before rotating the file.\ntype:uint64")?;
            log.put_str("filename_counters", &self.log_counters_filename, "Log file name for counters.\ntype:string")?;
            log.put_str("filename_samples", &self.log_samples_filename, "Log file name for samples.\ntype:string")?;
            Ok(())
        })?;
        Ok(())
    }
}
