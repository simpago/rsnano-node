use crate::{
    utils::{BufferWriter, Deserialize, FixedSizeSerialize, Serialize, Stream},
    Account, Block, BlockHash,
};
use primitive_types::U512;

/// This struct represents the data written into the pending (receivable) database table key
/// the receiving account and hash of the send block identify a pending db table entry
#[derive(Default, PartialEq, Eq, Debug, Clone)]
pub struct PendingKey {
    pub receiving_account: Account,
    pub send_block_hash: BlockHash,
}

impl PendingKey {
    pub fn new(receiving_account: Account, send_block_hash: BlockHash) -> Self {
        Self {
            receiving_account,
            send_block_hash,
        }
    }

    pub fn to_bytes(&self) -> [u8; 64] {
        let mut result = [0; 64];
        result[..32].copy_from_slice(self.receiving_account.as_bytes());
        result[32..].copy_from_slice(self.send_block_hash.as_bytes());
        result
    }

    pub fn for_send_block(block: &Block) -> Self {
        Self::new(block.link_field().unwrap_or_default().into(), block.hash())
    }

    pub fn for_receive_block(block: &Block) -> Self {
        Self::new(
            block.account_field().unwrap(),
            block.link_field().unwrap_or_default().into(),
        )
    }

    pub fn new_test_instance() -> Self {
        Self::new(Account::from(1), BlockHash::from(2))
    }
}

impl Serialize for PendingKey {
    fn serialize(&self, writer: &mut dyn BufferWriter) {
        self.receiving_account.serialize(writer);
        self.send_block_hash.serialize(writer);
    }
}

impl FixedSizeSerialize for PendingKey {
    fn serialized_size() -> usize {
        Account::serialized_size() + BlockHash::serialized_size()
    }
}

impl Deserialize for PendingKey {
    type Target = Self;

    fn deserialize(stream: &mut dyn Stream) -> anyhow::Result<Self::Target> {
        let account = Account::deserialize(stream)?;
        let hash = BlockHash::deserialize(stream)?;
        Ok(Self {
            receiving_account: account,
            send_block_hash: hash,
        })
    }
}

impl From<U512> for PendingKey {
    fn from(value: U512) -> Self {
        let buffer = value.to_big_endian();
        PendingKey::new(
            Account::from_slice(&buffer[..32]).unwrap(),
            BlockHash::from_slice(&buffer[32..]).unwrap(),
        )
    }
}

impl PartialOrd for PendingKey {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        if self.receiving_account == other.receiving_account {
            self.send_block_hash.partial_cmp(&other.send_block_hash)
        } else {
            self.receiving_account.partial_cmp(&other.receiving_account)
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::PendingKey;

    #[test]
    fn pending_key_sorting() {
        let one = PendingKey::new(1.into(), 2.into());
        let one_same = PendingKey::new(1.into(), 2.into());
        let two = PendingKey::new(1.into(), 3.into());
        let three = PendingKey::new(2.into(), 1.into());
        assert!(one < two);
        assert!(one < three);
        assert!(two < three);
        assert!(one == one_same);
        assert!(one != two);
    }
}
