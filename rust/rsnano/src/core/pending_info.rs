use std::mem::size_of;

use num::FromPrimitive;

use crate::utils::{Deserialize, Serialize, Stream};

use super::{Account, Amount, Epoch};

#[derive(PartialEq, Eq, Debug)]
pub struct PendingInfo {
    pub source: Account,
    pub amount: Amount,
    pub epoch: Epoch,
}

impl Default for PendingInfo {
    fn default() -> Self {
        Self {
            source: Default::default(),
            amount: Default::default(),
            epoch: Epoch::Epoch0,
        }
    }
}

impl PendingInfo {
    pub fn new(source: Account, amount: Amount, epoch: Epoch) -> Self {
        Self {
            source,
            amount,
            epoch,
        }
    }

    pub fn to_bytes(&self) -> [u8; 49] {
        let mut bytes = [0; 49];
        bytes[..32].copy_from_slice(self.source.as_bytes());
        bytes[32..48].copy_from_slice(&self.amount.to_be_bytes());
        bytes[48] = self.epoch as u8;
        bytes
    }
}

impl Serialize for PendingInfo {
    fn serialized_size() -> usize {
        Account::serialized_size() + Amount::serialized_size() + size_of::<u8>()
    }

    fn serialize(&self, stream: &mut dyn Stream) -> anyhow::Result<()> {
        self.source.serialize(stream)?;
        self.amount.serialize(stream)?;
        stream.write_u8(self.epoch as u8)
    }
}

impl Deserialize for PendingInfo {
    type Target = Self;

    fn deserialize(stream: &mut dyn Stream) -> anyhow::Result<Self::Target> {
        let source = Account::deserialize(stream)?;
        let amount = Amount::deserialize(stream)?;
        let epoch =
            FromPrimitive::from_u8(stream.read_u8()?).ok_or_else(|| anyhow!("invalid epoch"))?;
        Ok(Self {
            source,
            amount,
            epoch,
        })
    }
}
