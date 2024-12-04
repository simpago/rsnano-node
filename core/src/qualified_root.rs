use crate::{
    utils::{BufferWriter, Deserialize, FixedSizeSerialize, MutStreamAdapter, Serialize, Stream},
    BlockHash, Root,
};
use primitive_types::U512;
use std::hash::Hash;

#[derive(Default, Clone, PartialEq, Eq, Hash, Debug, serde::Serialize, serde::Deserialize)]
pub struct QualifiedRoot {
    pub root: Root,
    pub previous: BlockHash,
}

impl QualifiedRoot {
    pub fn new(root: Root, previous: BlockHash) -> Self {
        Self { root, previous }
    }

    pub fn to_bytes(&self) -> [u8; 64] {
        let mut buffer = [0; 64];
        let mut stream = MutStreamAdapter::new(&mut buffer);
        self.serialize(&mut stream);
        buffer
    }

    pub fn new_test_instance() -> Self {
        Self::new(Root::from(111), BlockHash::from(222))
    }
}

impl Serialize for QualifiedRoot {
    fn serialize(&self, writer: &mut dyn BufferWriter) {
        self.root.serialize(writer);
        self.previous.serialize(writer);
    }
}

impl FixedSizeSerialize for QualifiedRoot {
    fn serialized_size() -> usize {
        Root::serialized_size() + BlockHash::serialized_size()
    }
}

impl Deserialize for QualifiedRoot {
    type Target = Self;
    fn deserialize(stream: &mut dyn Stream) -> anyhow::Result<QualifiedRoot> {
        let root = Root::deserialize(stream)?;
        let previous = BlockHash::deserialize(stream)?;
        Ok(QualifiedRoot { root, previous })
    }
}

impl From<U512> for QualifiedRoot {
    fn from(value: U512) -> Self {
        let bytes = value.to_big_endian();
        let root = Root::from_slice(&bytes[..32]).unwrap();
        let previous = BlockHash::from_slice(&bytes[32..]).unwrap();
        QualifiedRoot { root, previous }
    }
}
