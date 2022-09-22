use super::{Message, MessageHeader, MessageType, MessageVisitor};
use crate::{
    utils::{Deserialize, Serialize, Stream},
    Account, Amount, NetworkConstants,
};
use anyhow::Result;
use num_traits::FromPrimitive;
use std::{any::Any, mem::size_of};

#[derive(Clone, Copy, PartialEq, Eq, FromPrimitive)]
#[repr(u8)]
pub enum BulkPullAccountFlags {
    PendingHashAndAmount = 0x0,
    PendingAddressOnly = 0x1,
    PendingHashAmountAndAddress = 0x2,
}

#[derive(Clone)]
pub struct BulkPullAccount {
    header: MessageHeader,
    pub account: Account,
    pub minimum_amount: Amount,
    pub flags: BulkPullAccountFlags,
}

impl BulkPullAccount {
    pub fn new(constants: &NetworkConstants) -> Self {
        Self {
            header: MessageHeader::new(constants, MessageType::BulkPullAccount),
            account: Account::new(),
            minimum_amount: Amount::zero(),
            flags: BulkPullAccountFlags::PendingHashAndAmount,
        }
    }

    pub fn with_header(header: MessageHeader) -> Self {
        Self {
            header,
            account: Account::new(),
            minimum_amount: Amount::zero(),
            flags: BulkPullAccountFlags::PendingHashAndAmount,
        }
    }

    pub fn from_stream(stream: &mut impl Stream, header: MessageHeader) -> Result<Self> {
        let mut msg = Self::with_header(header);
        msg.deserialize(stream)?;
        Ok(msg)
    }

    pub fn serialized_size() -> usize {
        Account::serialized_size() + Amount::serialized_size() + size_of::<BulkPullAccountFlags>()
    }

    pub fn deserialize(&mut self, stream: &mut impl Stream) -> Result<()> {
        debug_assert!(self.header.message_type() == MessageType::BulkPullAccount);
        self.account = Account::deserialize(stream)?;
        self.minimum_amount = Amount::deserialize(stream)?;
        self.flags = BulkPullAccountFlags::from_u8(stream.read_u8()?)
            .ok_or_else(|| anyhow!("invalid bulk pull account flag"))?;
        Ok(())
    }
}

impl Message for BulkPullAccount {
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
        self.header.serialize(stream)?;
        self.account.serialize(stream)?;
        self.minimum_amount.serialize(stream)?;
        stream.write_u8(self.flags as u8)
    }

    fn visit(&self, visitor: &mut dyn MessageVisitor) {
        visitor.bulk_pull_account(self)
    }

    fn clone_box(&self) -> Box<dyn Message> {
        Box::new(self.clone())
    }

    fn message_type(&self) -> MessageType {
        MessageType::BulkPullAccount
    }
}
