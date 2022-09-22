use anyhow::Result;

use crate::utils::TomlWriter;

pub struct StatConfig {
    /** If true, sampling of counters is enabled */
    pub sampling_enabled: bool,

    /** How many sample intervals to keep in the ring buffer */
    pub capacity: usize,

    /** Sample interval in milliseconds */
    pub interval: usize,

    /** How often to log sample array, in milliseconds. Default is 0 (no logging) */
    pub log_interval_samples: usize,

    /** How often to log counters, in milliseconds. Default is 0 (no logging) */
    pub log_interval_counters: usize,

    /** Maximum number of log outputs before rotating the file */
    pub log_rotation_count: usize,

    /** If true, write headers on each counter or samples writeout. The header contains log type and the current wall time. */
    pub log_headers: bool,

    /** Filename for the counter log  */
    pub log_counters_filename: String,

    /** Filename for the sampling log */
    pub log_samples_filename: String,
}

impl Default for StatConfig {
    fn default() -> Self {
        Self {
            sampling_enabled: false,
            capacity: 0,
            interval: 0,
            log_interval_samples: 0,
            log_interval_counters: 0,
            log_rotation_count: 100,
            log_headers: true,
            log_counters_filename: "counters.stat".to_string(),
            log_samples_filename: "samples.stat".to_string(),
        }
    }
}

impl StatConfig {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn serialize_toml(&self, toml: &mut dyn TomlWriter) -> Result<()> {
        toml.put_child("sampling", &mut |sampling| {
            sampling.put_bool(
                "enable",
                self.sampling_enabled,
                "Enable or disable sampling.\ntype:bool",
            )?;
            sampling.put_usize(
                "capacity",
                self.capacity,
                "How many sample intervals to keep in the ring buffer.\ntype:uint64",
            )?;
            sampling.put_usize(
                "interval",
                self.interval,
                "Sample interval.\ntype:milliseconds",
            )?;
            Ok(())
        })?;

        toml.put_child("log", &mut |log|{
            log.put_bool("headers", self.log_headers, "If true, write headers on each counter or samples writeout.\nThe header contains log type and the current wall time.\ntype:bool")?;
            log.put_usize("interval_counters", self.log_interval_counters, "How often to log counters. 0 disables logging.\ntype:milliseconds")?;
            log.put_usize("interval_samples", self.log_interval_samples, "How often to log samples. 0 disables logging.\ntype:milliseconds")?;
            log.put_usize("rotation_count", self.log_rotation_count, "Maximum number of log outputs before rotating the file.\ntype:uint64")?;
            log.put_str("filename_counters", &self.log_counters_filename, "Log file name for counters.\ntype:string")?;
            log.put_str("filename_samples", &self.log_samples_filename, "Log file name for samples.\ntype:string")?;
            Ok(())
        })?;
        Ok(())
    }
}
