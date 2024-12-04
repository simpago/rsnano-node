use crate::{
    utils::{
        BufferWriter, Deserialize, FixedSizeSerialize, MutStreamAdapter, Serialize, Stream,
        StreamExt,
    },
    BlockHash,
};

#[derive(Default, PartialEq, Eq, Debug, Clone)]
pub struct ConfirmationHeightInfo {
    pub height: u64,
    pub frontier: BlockHash,
}

impl ConfirmationHeightInfo {
    pub fn new(height: u64, frontier: BlockHash) -> Self {
        Self { height, frontier }
    }

    pub fn to_bytes(&self) -> [u8; 40] {
        let mut buffer = [0; 40];
        let mut stream = MutStreamAdapter::new(&mut buffer);
        self.serialize(&mut stream);
        buffer
    }

    pub fn test_instance() -> Self {
        Self {
            height: 42,
            frontier: BlockHash::from(7),
        }
    }
}

impl Serialize for ConfirmationHeightInfo {
    fn serialize(&self, writer: &mut dyn BufferWriter) {
        writer.write_u64_ne_safe(self.height);
        self.frontier.serialize(writer);
    }
}

impl FixedSizeSerialize for ConfirmationHeightInfo {
    fn serialized_size() -> usize {
        std::mem::size_of::<u64>() + BlockHash::serialized_size()
    }
}

impl Deserialize for ConfirmationHeightInfo {
    type Target = Self;
    fn deserialize(stream: &mut dyn Stream) -> anyhow::Result<Self> {
        let height = stream.read_u64_ne()?;
        let frontier = BlockHash::deserialize(stream)?;
        Ok(Self { height, frontier })
    }
}
