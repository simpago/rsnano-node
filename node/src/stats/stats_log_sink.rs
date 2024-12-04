use anyhow::Result;
use chrono::{DateTime, Local};
use std::{any::Any, fs::File, io::Write, path::PathBuf, time::SystemTime};

pub trait StatsLogSink {
    /// Called before logging starts
    fn begin(&mut self) -> Result<()>;

    /// Called after logging is completed
    fn finalize(&mut self);

    /// Write a header enrty to the log
    fn write_header(&mut self, header: &str, walltime: SystemTime) -> Result<()>;

    /// Write a counter or sampling entry to the log. Some log sinks may support writing histograms as well.
    fn write_counter_entry(
        &mut self,
        time: SystemTime,
        entry_type: &str,
        detail: &str,
        dir: &str,
        value: u64,
    ) -> Result<()>;

    fn write_sampler_entry(
        &mut self,
        time: SystemTime,
        sample: &str,
        values: Vec<i64>,
        expected_min_max: (i64, i64),
    ) -> Result<()>;

    /// Rotates the log (e.g. empty file). This is a no-op for sinks where rotation is not supported.
    fn rotate(&mut self) -> Result<()>;

    /// Returns a reference to the log entry counter
    fn entries(&self) -> usize;

    fn inc_entries(&mut self);

    /// Returns the string representation of the log. If not supported, an empty string is returned.
    fn to_string(&self) -> String;

    /// Returns the object representation of the log result. The type depends on the sink used.
    /// returns Object, or nullptr if no object result is available.
    fn to_object(&self) -> Option<&dyn Any>;
}

/// File sink with rotation support. This writes one counter per line and does not include histogram values.
pub struct StatFileWriter {
    filename: PathBuf,
    file: File,
    log_entries: usize,
}

impl StatFileWriter {
    pub fn new(filename: impl Into<PathBuf>) -> Result<Self> {
        let filename = filename.into();
        let file = File::create(filename.clone())?;
        Ok(Self {
            filename,
            file,
            log_entries: 0,
        })
    }
}

impl StatsLogSink for StatFileWriter {
    fn begin(&mut self) -> Result<()> {
        Ok(())
    }

    fn finalize(&mut self) {}

    fn write_header(&mut self, header: &str, walltime: SystemTime) -> Result<()> {
        let local = DateTime::<Local>::from(walltime);
        let local_fmt = local.format("%Y.%m.%d %H:%M:%S");
        writeln!(&mut self.file, "{header},{local_fmt}")?;
        Ok(())
    }

    fn write_counter_entry(
        &mut self,
        time: SystemTime,
        entry_type: &str,
        detail: &str,
        dir: &str,
        value: u64,
    ) -> Result<()> {
        let now = DateTime::<Local>::from(time).format("%H:%M:%S");
        writeln!(&mut self.file, "{now},{entry_type},{detail},{dir},{value}")?;
        Ok(())
    }

    fn write_sampler_entry(
        &mut self,
        time: SystemTime,
        sample: &str,
        values: Vec<i64>,
        _expected_min_max: (i64, i64),
    ) -> Result<()> {
        let time: chrono::DateTime<Local> = time.into();
        write!(&mut self.file, "{},{sample}", time.format("%H:%M:%S"))?;

        for value in values {
            write!(&mut self.file, ",{}", value)?;
        }

        writeln!(&mut self.file, "")?;

        Ok(())
    }

    fn rotate(&mut self) -> Result<()> {
        self.file = File::create(self.filename.clone())?;
        self.log_entries = 0;
        Ok(())
    }

    fn entries(&self) -> usize {
        self.log_entries
    }

    fn inc_entries(&mut self) {
        self.log_entries += 1;
    }

    fn to_string(&self) -> String {
        String::new()
    }

    fn to_object(&self) -> Option<&dyn Any> {
        None
    }
}

pub struct StatsJsonWriterV2 {
    tree: serde_json::Map<String, serde_json::Value>,
    entries: Vec<serde_json::Value>,
    log_entries: usize,
}

impl StatsJsonWriterV2 {
    pub fn new() -> Self {
        Self {
            tree: Default::default(),
            entries: Default::default(),
            log_entries: 0,
        }
    }

    pub fn add(&mut self, key: impl Into<String>, value: u64) {
        self.tree
            .insert(key.into(), serde_json::Value::String(value.to_string()));
    }

    pub fn finish(self) -> serde_json::Value {
        serde_json::Value::Object(self.tree)
    }
}

impl Default for StatsJsonWriterV2 {
    fn default() -> Self {
        Self::new()
    }
}

impl StatsLogSink for StatsJsonWriterV2 {
    fn begin(&mut self) -> Result<()> {
        self.tree.clear();
        Ok(())
    }

    fn finalize(&mut self) {
        let empty_entries = Vec::new();
        let entries = std::mem::replace(&mut self.entries, empty_entries);
        self.tree
            .insert("entries".to_owned(), serde_json::Value::Array(entries));
    }

    fn write_header(&mut self, header: &str, walltime: SystemTime) -> Result<()> {
        let now = DateTime::<Local>::from(walltime);
        self.tree.insert(
            "type".to_owned(),
            serde_json::Value::String(header.to_owned()),
        );
        self.tree.insert(
            "created".to_owned(),
            serde_json::Value::String(now.format("%Y.%m.%d %H:%M:%S").to_string()),
        );
        Ok(())
    }

    fn write_counter_entry(
        &mut self,
        time: SystemTime,
        entry_type: &str,
        detail: &str,
        dir: &str,
        value: u64,
    ) -> Result<()> {
        let mut entry = serde_json::Map::new();
        entry.insert(
            "time".to_owned(),
            serde_json::Value::String(DateTime::<Local>::from(time).format("%H:%M:%S").to_string()),
        );
        entry.insert(
            "type".to_owned(),
            serde_json::Value::String(entry_type.to_owned()),
        );
        entry.insert(
            "detail".to_owned(),
            serde_json::Value::String(detail.to_owned()),
        );
        entry.insert("dir".to_owned(), serde_json::Value::String(dir.to_owned()));
        entry.insert(
            "value".to_owned(),
            serde_json::Value::String(value.to_string()),
        );
        self.entries.push(serde_json::Value::Object(entry));
        Ok(())
    }

    fn rotate(&mut self) -> Result<()> {
        Ok(())
    }

    fn entries(&self) -> usize {
        self.log_entries
    }

    fn inc_entries(&mut self) {
        self.log_entries += 1;
    }

    fn to_string(&self) -> String {
        serde_json::Value::Object(self.tree.clone()).to_string()
    }

    fn to_object(&self) -> Option<&dyn Any> {
        None
    }

    fn write_sampler_entry(
        &mut self,
        time: SystemTime,
        sample: &str,
        values: Vec<i64>,
        expected_min_max: (i64, i64),
    ) -> Result<()> {
        let time: chrono::DateTime<Local> = time.into();
        let mut entry = serde_json::Map::new();
        entry.insert(
            "time".to_owned(),
            serde_json::Value::String(time.format("%H:%M:%S").to_string()),
        );
        entry.insert(
            "sample".to_owned(),
            serde_json::Value::String(sample.to_owned()),
        );
        entry.insert(
            "min".to_owned(),
            serde_json::Value::String(expected_min_max.0.to_string()),
        );
        entry.insert(
            "max".to_owned(),
            serde_json::Value::String(expected_min_max.1.to_string()),
        );

        let mut values_tree = Vec::new();
        for value in values {
            values_tree.push(serde_json::Value::String(value.to_string()));
        }
        entry.insert("values".to_owned(), serde_json::Value::Array(values_tree));
        self.entries.push(serde_json::Value::Object(entry));
        Ok(())
    }
}
