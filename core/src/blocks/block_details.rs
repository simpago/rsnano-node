use crate::{
    utils::{BufferWriter, Serialize, Stream},
    Epoch,
};
use anyhow::Result;
use num::FromPrimitive;

use super::BlockSubType;

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct BlockDetails {
    pub epoch: Epoch,
    pub is_send: bool,
    pub is_receive: bool,
    pub is_epoch: bool,
}

impl BlockDetails {
    pub fn new(epoch: Epoch, is_send: bool, is_receive: bool, is_epoch: bool) -> Self {
        Self {
            epoch,
            is_send,
            is_receive,
            is_epoch,
        }
    }

    pub const fn serialized_size() -> usize {
        1
    }

    pub fn deserialize(stream: &mut dyn Stream) -> Result<BlockDetails> {
        BlockDetails::unpack(stream.read_u8()?)
    }

    pub fn packed(&self) -> u8 {
        let mut result = self.epoch as u8;
        if self.is_send {
            result |= 0b1000_0000;
        }
        if self.is_receive {
            result |= 0b0100_0000;
        }
        if self.is_epoch {
            result |= 0b0010_0000;
        }

        result
    }

    pub fn unpack(value: u8) -> Result<Self> {
        let epoch_mask = 0b0001_1111u8;
        let epoch_value = value & epoch_mask;
        let epoch = match FromPrimitive::from_u8(epoch_value) {
            Some(e) => e,
            None => bail!("unknown epoch value: {}", epoch_value),
        };

        Ok(BlockDetails {
            epoch,
            is_send: (0b1000_0000 & value) != 0,
            is_receive: (0b0100_0000 & value) != 0,
            is_epoch: (0b0010_0000 & value) != 0,
        })
    }

    pub fn subtype(&self) -> BlockSubType {
        if self.is_send {
            BlockSubType::Send
        } else if self.is_receive {
            BlockSubType::Receive
        } else if self.is_epoch {
            BlockSubType::Epoch
        } else {
            BlockSubType::Change
        }
    }

    pub fn subtype_str(&self) -> &'static str {
        self.subtype().as_str()
    }
}

impl Serialize for BlockDetails {
    fn serialize(&self, stream: &mut dyn BufferWriter) {
        stream.write_u8_safe(self.packed())
    }
}

#[cfg(test)]
mod test {
    use crate::utils::MemoryStream;

    use super::*;

    #[test]
    fn test_block_details() {
        let details_send = BlockDetails::new(Epoch::Epoch0, true, false, false);
        assert_eq!(details_send.is_send, true);
        assert_eq!(details_send.is_receive, false);
        assert_eq!(details_send.is_epoch, false);
        assert_eq!(details_send.epoch, Epoch::Epoch0);

        let details_receive = BlockDetails::new(Epoch::Epoch1, false, true, false);
        assert_eq!(details_receive.is_send, false);
        assert_eq!(details_receive.is_receive, true);
        assert_eq!(details_receive.is_epoch, false);
        assert_eq!(details_receive.epoch, Epoch::Epoch1);

        let details_epoch = BlockDetails::new(Epoch::Epoch2, false, false, true);
        assert_eq!(details_epoch.is_send, false);
        assert_eq!(details_epoch.is_receive, false);
        assert_eq!(details_epoch.is_epoch, true);
        assert_eq!(details_epoch.epoch, Epoch::Epoch2);

        let details_none = BlockDetails::new(Epoch::Unspecified, false, false, false);
        assert_eq!(details_none.is_send, false);
        assert_eq!(details_none.is_receive, false);
        assert_eq!(details_none.is_epoch, false);
        assert_eq!(details_none.epoch, Epoch::Unspecified);
    }

    #[test]
    fn test_pack_and_unpack() {
        let details_send = BlockDetails::new(Epoch::Epoch0, true, false, false);
        assert_eq!(details_send.packed(), 0b1000_0010);
        assert_eq!(
            BlockDetails::unpack(details_send.packed()).unwrap(),
            details_send
        );

        let details_receive = BlockDetails::new(Epoch::Epoch1, false, true, false);
        assert_eq!(details_receive.packed(), 0b0100_0011);
        assert_eq!(
            BlockDetails::unpack(details_receive.packed()).unwrap(),
            details_receive
        );

        let details_epoch = BlockDetails::new(Epoch::Epoch2, false, false, true);
        assert_eq!(details_epoch.packed(), 0b0010_0100);
        assert_eq!(
            BlockDetails::unpack(details_epoch.packed()).unwrap(),
            details_epoch
        );

        let details_none = BlockDetails::new(Epoch::Unspecified, false, false, false);
        assert_eq!(details_none.packed(), 0b0000_0001);
        assert_eq!(
            BlockDetails::unpack(details_none.packed()).unwrap(),
            details_none
        );
    }

    #[test]
    fn serialize() {
        let details = BlockDetails::new(Epoch::Epoch2, false, true, false);
        let mut stream = MemoryStream::new();
        details.serialize(&mut stream);
        let deserialized = BlockDetails::deserialize(&mut stream).unwrap();
        assert_eq!(deserialized, details);
    }
}
