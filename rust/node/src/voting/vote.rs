use anyhow::Result;
use rsnano_core::{
    sign_message,
    utils::{Deserialize, PropertyTreeWriter, SerdePropertyTree, Serialize, Stream},
    validate_message, Account, BlockHash, BlockHashBuilder, FullHash, RawKey, Signature,
};
use std::time::Duration;

#[derive(Clone)]
pub struct Vote {
    pub timestamp: u64,

    // Account that's voting
    pub voting_account: Account,

    // Signature of timestamp + block hashes
    pub signature: Signature,

    // The hashes for which this vote directly covers
    pub hashes: Vec<BlockHash>,
}

static HASH_PREFIX: &str = "vote ";

impl Vote {
    pub fn null() -> Self {
        Self {
            timestamp: 0,
            voting_account: Account::zero(),
            signature: Signature::new(),
            hashes: Vec::new(),
        }
    }

    pub fn new(
        account: Account,
        prv: &RawKey,
        timestamp: u64,
        duration: u8,
        hashes: Vec<BlockHash>,
    ) -> Result<Self> {
        let mut result = Self {
            voting_account: account,
            timestamp: packed_timestamp(timestamp, duration),
            signature: Signature::new(),
            hashes,
        };
        result.signature =
            sign_message(prv, &result.voting_account.into(), result.hash().as_bytes());
        Ok(result)
    }

    /// Returns the timestamp of the vote (with the duration bits masked, set to zero)
    /// If it is a final vote, all the bits including duration bits are returned as they are, all FF
    pub fn timestamp(&self) -> u64 {
        if self.timestamp == u64::MAX {
            self.timestamp //final vote
        } else {
            self.timestamp & TIMESTAMP_MASK
        }
    }

    pub fn duration_bits(&self) -> u8 {
        // Duration field is specified in the 4 low-order bits of the timestamp.
        // This makes the timestamp have a minimum granularity of 16ms
        // The duration is specified as 2^(duration + 4) giving it a range of 16-524,288ms in power of two increments
        let result = self.timestamp & !TIMESTAMP_MASK;
        result as u8
    }

    pub fn duration(&self) -> Duration {
        Duration::from_millis(1 << (self.duration_bits() + 4))
    }

    pub fn vote_hashes_string(&self) -> String {
        let mut result = String::new();
        for h in self.hashes.iter() {
            result.push_str(&h.to_string());
            result.push_str(", ");
        }
        result
    }

    pub fn serialize_json(&self, writer: &mut dyn PropertyTreeWriter) -> Result<()> {
        writer.put_string("account", &self.voting_account.encode_account())?;
        writer.put_string("signature", &self.signature.encode_hex())?;
        writer.put_string("sequence", &self.timestamp().to_string())?;
        writer.put_string("timestamp", &self.timestamp().to_string())?;
        writer.put_string("duration", &self.duration_bits().to_string())?;
        let mut blocks_tree = writer.new_writer();
        for hash in &self.hashes {
            let mut entry = writer.new_writer();
            entry.put_string("", &hash.to_string())?;
            blocks_tree.push_back("", entry.as_ref());
        }
        writer.add_child("blocks", blocks_tree.as_ref());
        Ok(())
    }

    pub fn to_json(&self) -> anyhow::Result<String> {
        let mut ptree = SerdePropertyTree::new();
        self.serialize_json(&mut ptree)?;
        Ok(ptree.to_json())
    }

    pub fn hash(&self) -> BlockHash {
        let mut builder = BlockHashBuilder::new().update(HASH_PREFIX);

        for hash in &self.hashes {
            builder = builder.update(hash.as_bytes())
        }

        builder.update(self.timestamp.to_ne_bytes()).build()
    }

    pub fn serialize(&self, stream: &mut dyn Stream) -> Result<()> {
        self.voting_account.serialize(stream)?;
        self.signature.serialize(stream)?;
        stream.write_bytes(&self.timestamp.to_le_bytes())?;
        for hash in &self.hashes {
            hash.serialize(stream)?;
        }
        Ok(())
    }

    pub fn deserialize(&mut self, stream: &mut impl Stream) -> Result<()> {
        self.voting_account = Account::deserialize(stream)?;
        self.signature = Signature::deserialize(stream)?;
        let mut buffer = [0; 8];
        stream.read_bytes(&mut buffer, 8)?;
        self.timestamp = u64::from_ne_bytes(buffer);
        self.hashes = Vec::new();
        while stream.in_avail()? > 0 {
            self.hashes.push(BlockHash::deserialize(stream)?);
        }
        Ok(())
    }

    pub fn validate(&self) -> Result<()> {
        validate_message(
            &self.voting_account.into(),
            self.hash().as_bytes(),
            &self.signature,
        )
    }

    pub fn serialized_size(count: usize) -> usize {
        Account::serialized_size()
        + Signature::serialized_size()
        + std::mem::size_of::<u64>() // timestamp
        + (BlockHash::serialized_size() * count)
    }
}

impl PartialEq for Vote {
    fn eq(&self, other: &Self) -> bool {
        self.timestamp == other.timestamp
            && self.voting_account == other.voting_account
            && self.signature == other.signature
            && self.hashes == other.hashes
    }
}

impl FullHash for Vote {
    fn full_hash(&self) -> BlockHash {
        BlockHashBuilder::new()
            .update(self.hash().as_bytes())
            .update(self.voting_account.as_bytes())
            .update(self.signature.as_bytes())
            .build()
    }
}

const DURATION_MAX: u8 = 0x0F;
const TIMESTAMP_MAX: u64 = 0xFFFF_FFFF_FFFF_FFF0;
const TIMESTAMP_MASK: u64 = 0xFFFF_FFFF_FFFF_FFF0;

fn packed_timestamp(timestamp: u64, duration: u8) -> u64 {
    debug_assert!(duration <= DURATION_MAX);
    debug_assert!(timestamp != TIMESTAMP_MAX || duration == DURATION_MAX);
    (timestamp & TIMESTAMP_MASK) | (duration as u64)
}
