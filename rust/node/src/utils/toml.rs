use anyhow::Result;
use rsnano_core::utils::{TomlArrayWriter, TomlWriter};
use std::{io::BufRead, path::Path};
use toml_edit::Document;

#[derive(Default)]
pub struct TomlConfig {
    doc: Document,
}

impl TomlConfig {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn to_string(&self) -> String {
        self.doc.to_string()
    }

    pub fn to_string_with_comments(&self, comment_values: bool) -> String {
        let mut ss_processed = String::new();

        // Convert the TOML value to a string
        let toml_string = self.doc.to_string();

        // Use a buffered reader to read the TOML string line by line
        let reader = std::io::BufReader::new(toml_string.as_bytes());

        for line in reader.lines() {
            let mut line = line.unwrap();
            if !line.is_empty() && !line.starts_with('[') {
                if line.starts_with('#') {
                    line = format!("\t{}", line);
                } else {
                    line = if comment_values {
                        format!("\t# {}", line)
                    } else {
                        format!("\t{}", line)
                    };
                }
            }
            ss_processed.push_str(&line);
            ss_processed.push('\n');
        }

        ss_processed
    }

    pub fn write(&self, file: impl AsRef<Path>) -> Result<()> {
        std::fs::write(file, self.to_string().as_bytes())?;
        Ok(())
    }
}

impl TomlWriter for TomlConfig {
    fn put_u16(&mut self, key: &str, value: u16, _documentation: &str) -> Result<()> {
        self.doc[key] = toml_edit::value(value as i64);
        Ok(())
    }

    fn put_u32(&mut self, key: &str, value: u32, _documentation: &str) -> Result<()> {
        self.doc[key] = toml_edit::value(value as i64);
        Ok(())
    }

    fn put_u64(&mut self, key: &str, value: u64, _documentation: &str) -> Result<()> {
        self.doc[key] = toml_edit::value(value as i64);
        Ok(())
    }

    fn put_i64(&mut self, key: &str, value: i64, _documentation: &str) -> Result<()> {
        self.doc[key] = toml_edit::value(value);
        Ok(())
    }

    fn put_str(&mut self, key: &str, value: &str, _documentation: &str) -> Result<()> {
        self.doc[key] = toml_edit::value(value);
        Ok(())
    }

    fn put_bool(&mut self, key: &str, value: bool, _documentation: &str) -> Result<()> {
        self.doc[key] = toml_edit::value(value);
        Ok(())
    }

    fn put_usize(&mut self, key: &str, value: usize, _documentation: &str) -> Result<()> {
        self.doc[key] = toml_edit::value(value as i64);
        Ok(())
    }

    fn put_f64(&mut self, key: &str, value: f64, _documentation: &str) -> Result<()> {
        self.doc[key] = toml_edit::value(value);
        Ok(())
    }

    fn create_array(
        &mut self,
        key: &str,
        _documentation: &str,
        f: &mut dyn FnMut(&mut dyn TomlArrayWriter) -> Result<()>,
    ) -> Result<()> {
        let mut array = TomlConfigArray::new();
        f(&mut array)?;
        self.doc[key] = toml_edit::Item::Value(toml_edit::Value::Array(array.array));
        Ok(())
    }

    fn put_child(
        &mut self,
        key: &str,
        f: &mut dyn FnMut(&mut dyn TomlWriter) -> Result<()>,
    ) -> Result<()> {
        let mut child = TomlConfig::new();
        f(&mut child)?;
        self.doc[key] = toml_edit::Item::Table(child.doc.as_table().clone());
        Ok(())
    }
}

#[derive(Default)]
pub struct TomlConfigArray {
    array: toml_edit::Array,
}

impl TomlConfigArray {
    pub fn new() -> Self {
        Default::default()
    }
}

impl TomlArrayWriter for TomlConfigArray {
    fn push_back_str(&mut self, value: &str) -> Result<()> {
        self.array.push(value);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_toml_doc() -> Result<()> {
        let mut toml = TomlConfig::new();
        toml.put_bool("bool_test", true, "ignored")?;
        toml.put_i64("i64_test", 123, "ignored")?;
        toml.create_array("array_test", "ignored", &mut |a| {
            a.push_back_str("hello")?;
            a.push_back_str("world")?;
            Ok(())
        })?;
        toml.put_child("child_test", &mut |c| {
            c.put_bool("child_bool", false, "ignored")?;
            c.put_child("sub_child", &mut |sc| {
                sc.put_i64("sub_child_i64", 999, "ignored")?;
                Ok(())
            })?;
            Ok(())
        })?;

        let result = toml.doc.to_string();
        assert_eq!(
            result,
            r#"bool_test = true
i64_test = 123
array_test = ["hello", "world"]

[child_test]
child_bool = false

[child_test.sub_child]
sub_child_i64 = 999
"#
        );
        Ok(())
    }
}
