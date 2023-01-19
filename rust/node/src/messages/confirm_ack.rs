use crate::{
    config::NetworkConstants,
    messages::MessageType,
    voting::{Vote, VoteUniquer},
};
use anyhow::Result;
use rsnano_core::{utils::Stream, BlockType};
use std::{
    any::Any,
    fmt::{Debug, Display},
    sync::{Arc, RwLock},
};

use super::{Message, MessageHeader, MessageVisitor};

#[derive(Clone)]
pub struct ConfirmAck {
    header: MessageHeader,
    vote: Option<Arc<RwLock<Vote>>>,
}

impl ConfirmAck {
    pub const HASHES_MAX: usize = 12;

    pub fn new(constants: &NetworkConstants, vote: Arc<RwLock<Vote>>) -> Self {
        let mut header = MessageHeader::new(constants, MessageType::ConfirmAck);
        header.set_block_type(BlockType::NotABlock);
        let vote_lk = vote.read().unwrap();
        debug_assert!(vote_lk.hashes.len() < 16);
        header.set_count(vote_lk.hashes.len() as u8);
        drop(vote_lk);

        Self {
            header,
            vote: Some(vote),
        }
    }
    pub fn with_header(
        header: MessageHeader,
        stream: &mut impl Stream,
        uniquer: Option<&VoteUniquer>,
    ) -> Result<Self> {
        let mut vote = Vote::null();
        vote.deserialize(stream)?;
        let mut vote = Arc::new(RwLock::new(vote));

        if let Some(uniquer) = uniquer {
            vote = uniquer.unique(&vote);
        }

        Ok(Self {
            header,
            vote: Some(vote),
        })
    }

    pub fn vote(&self) -> Option<&Arc<RwLock<Vote>>> {
        self.vote.as_ref()
    }

    pub fn serialized_size(count: u8) -> usize {
        Vote::serialized_size(count as usize)
    }
}

impl Message for ConfirmAck {
    fn header(&self) -> &MessageHeader {
        &self.header
    }

    fn set_header(&mut self, header: &MessageHeader) {
        self.header = header.clone();
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn serialize(&self, stream: &mut dyn Stream) -> Result<()> {
        debug_assert!(
            self.header().block_type() == BlockType::NotABlock
                || self.header().block_type() == BlockType::LegacySend
                || self.header().block_type() == BlockType::LegacyReceive
                || self.header().block_type() == BlockType::LegacyOpen
                || self.header.block_type() == BlockType::LegacyChange
                || self.header.block_type() == BlockType::State
        );
        self.header().serialize(stream)?;
        self.vote().unwrap().read().unwrap().serialize(stream)
    }

    fn visit(&self, visitor: &mut dyn MessageVisitor) {
        visitor.confirm_ack(self)
    }

    fn clone_box(&self) -> Box<dyn Message> {
        Box::new(self.clone())
    }

    fn message_type(&self) -> MessageType {
        MessageType::ConfirmAck
    }
}

impl PartialEq for ConfirmAck {
    fn eq(&self, other: &Self) -> bool {
        if self.vote.is_some() != other.vote.is_some() {
            return false;
        }

        if let Some(v1) = &self.vote {
            if let Some(v2) = &other.vote {
                let lk1 = v1.read().unwrap();
                let lk2 = v2.read().unwrap();
                if *lk1 != *lk2 {
                    return false;
                }
            }
        }
        self.header == other.header
    }
}

impl Debug for ConfirmAck {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConfirmAck")
            .field("header", &self.header)
            .finish()
    }
}

impl Display for ConfirmAck {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.header, f)?;
        if let Some(vote) = &self.vote {
            write!(
                f,
                "\n{}",
                vote.read()
                    .unwrap()
                    .to_json()
                    .map_err(|_| std::fmt::Error)?
            )?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use rsnano_core::{utils::MemoryStream, BlockHash, KeyPair};

    use crate::config::NetworkConstants;

    use super::*;

    #[test]
    fn serialize() -> Result<()> {
        let constants = &NetworkConstants::empty();
        let keys = KeyPair::new();
        let mut hashes = Vec::new();
        for i in 0..ConfirmAck::HASHES_MAX {
            hashes.push(BlockHash::from(i as u64))
        }
        let vote = Vote::new(keys.public_key().into(), &keys.private_key(), 0, 0, hashes)?;
        let vote = Arc::new(RwLock::new(vote));
        let confirm1 = ConfirmAck::new(constants, vote);

        let mut stream = MemoryStream::new();
        confirm1.serialize(&mut stream)?;
        let header = MessageHeader::from_stream(&mut stream)?;
        let confirm2 = ConfirmAck::with_header(header, &mut stream, None)?;
        assert_eq!(confirm1, confirm2);
        Ok(())
    }
}
