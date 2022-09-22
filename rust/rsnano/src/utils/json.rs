use anyhow::Result;
use std::any::Any;
#[cfg(test)]
use std::collections::HashMap;

pub trait PropertyTreeReader {
    fn get_string(&self, path: &str) -> Result<String>;
}

pub trait PropertyTreeWriter {
    fn clear(&mut self) -> Result<()>;
    fn put_string(&mut self, path: &str, value: &str) -> Result<()>;
    fn put_u64(&mut self, path: &str, value: u64) -> Result<()>;
    fn new_writer(&self) -> Box<dyn PropertyTreeWriter>;
    fn push_back(&mut self, path: &str, value: &dyn PropertyTreeWriter);
    fn add_child(&mut self, path: &str, value: &dyn PropertyTreeWriter);
    fn put_child(&mut self, path: &str, value: &dyn PropertyTreeWriter);
    fn add(&mut self, path: &str, value: &str) -> Result<()>;
    fn as_any(&self) -> &dyn Any;
    fn to_json(&self) -> String;
}

/// Note: Once FfiPropertyTree is not used anymore we can return
/// the tree unboxed
pub(crate) fn create_property_tree() -> Box<dyn PropertyTreeWriter> {
    crate::ffi::create_ffi_property_tree()
}

#[cfg(test)]
pub struct TestPropertyTree {
    properties: HashMap<String, String>,
}

#[cfg(test)]
impl TestPropertyTree {
    pub fn new() -> Self {
        Self {
            properties: HashMap::new(),
        }
    }
}

#[cfg(test)]
impl PropertyTreeReader for TestPropertyTree {
    fn get_string(&self, path: &str) -> Result<String> {
        self.properties
            .get(path)
            .cloned()
            .ok_or_else(|| anyhow!("path not found"))
    }
}

#[cfg(test)]
impl PropertyTreeWriter for TestPropertyTree {
    fn put_string(&mut self, path: &str, value: &str) -> Result<()> {
        self.properties.insert(path.to_owned(), value.to_owned());
        Ok(())
    }

    fn put_u64(&mut self, _path: &str, _value: u64) -> Result<()> {
        todo!()
    }

    fn new_writer(&self) -> Box<dyn PropertyTreeWriter> {
        todo!()
    }

    fn push_back(&mut self, _path: &str, _value: &dyn PropertyTreeWriter) {
        todo!()
    }

    fn add_child(&mut self, _path: &str, _value: &dyn PropertyTreeWriter) {
        todo!()
    }

    fn add(&mut self, _path: &str, _value: &str) -> Result<()> {
        todo!()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn clear(&mut self) -> Result<()> {
        todo!()
    }

    fn put_child(&mut self, _path: &str, _value: &dyn PropertyTreeWriter) {
        todo!()
    }

    fn to_json(&self) -> String {
        todo!()
    }
}

pub struct SerdePropertyTree {
    value: serde_json::Value,
}

impl SerdePropertyTree {
    pub fn parse(s: &str) -> Result<Self> {
        Ok(Self {
            value: serde_json::from_str(s)?,
        })
    }
}

impl PropertyTreeReader for SerdePropertyTree {
    fn get_string(&self, path: &str) -> Result<String> {
        match self.value.get(path) {
            Some(v) => match v {
                serde_json::Value::String(s) => Ok(s.to_owned()),
                _ => Err(anyhow!("not a string value")),
            },
            None => Err(anyhow!("could not find path")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn property_not_found() {
        let tree = TestPropertyTree::new();
        assert!(tree.get_string("DoesNotExist").is_err());
    }

    #[test]
    fn set_string_property() {
        let mut tree = TestPropertyTree::new();
        tree.put_string("foo", "bar").unwrap();
        assert_eq!(tree.get_string("foo").unwrap(), "bar");
    }
}
