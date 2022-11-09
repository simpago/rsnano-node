use super::{Message, MessageHeader, MessageType, MessageVisitor};
use crate::{
    config::NetworkConstants,
    core::{Account, Signature},
    utils::{Deserialize, Serialize, Stream},
};
use anyhow::Result;
use std::any::Any;

#[derive(Clone)]
pub struct NodeIdHandshake {
    header: MessageHeader,
    pub query: Option<[u8; 32]>,
    pub response: Option<(Account, Signature)>,
}

impl NodeIdHandshake {
    pub fn new(
        constants: &NetworkConstants,
        query: Option<[u8; 32]>,
        response: Option<(Account, Signature)>,
    ) -> Self {
        let mut header = MessageHeader::new(constants, MessageType::NodeIdHandshake);

        if query.is_some() {
            header.set_flag(Self::QUERY_FLAG as u8);
        }

        if response.is_some() {
            header.set_flag(Self::RESPONSE_FLAG as u8);
        }

        Self {
            header,
            query,
            response,
        }
    }

    pub fn with_header(header: MessageHeader) -> Self {
        Self {
            header,
            query: None,
            response: None,
        }
    }

    pub fn from_stream(stream: &mut dyn Stream, header: MessageHeader) -> Result<Self> {
        let mut request = NodeIdHandshake::with_header(header);
        request.deserialize(stream)?;
        Ok(request)
    }

    const QUERY_FLAG: usize = 0;
    const RESPONSE_FLAG: usize = 1;

    fn is_query(header: &MessageHeader) -> bool {
        header.message_type() == MessageType::NodeIdHandshake
            && header.test_extension(NodeIdHandshake::QUERY_FLAG)
    }

    fn is_response(header: &MessageHeader) -> bool {
        header.message_type() == MessageType::NodeIdHandshake
            && header.test_extension(NodeIdHandshake::RESPONSE_FLAG)
    }

    pub fn deserialize(&mut self, stream: &mut dyn Stream) -> Result<()> {
        debug_assert!(self.header.message_type() == MessageType::NodeIdHandshake);
        if Self::is_query(&self.header) {
            let mut buffer = [0u8; 32];
            stream.read_bytes(&mut buffer, 32)?;
            self.query = Some(buffer);
        }

        if Self::is_response(&self.header) {
            let response_account = Account::deserialize(stream)?;
            let response_signature = Signature::deserialize(stream)?;
            self.response = Some((response_account, response_signature));
        }

        Ok(())
    }

    pub fn serialized_size(header: &MessageHeader) -> usize {
        let mut size = 0;
        if Self::is_query(header) {
            size += 32
        }
        if Self::is_response(header) {
            size += Account::serialized_size() + Signature::serialized_size()
        }
        size
    }
}

impl Message for NodeIdHandshake {
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
        if let Some(query) = &self.query {
            stream.write_bytes(query)?;
        }
        if let Some((acc, sig)) = &self.response {
            acc.serialize(stream)?;
            sig.serialize(stream)?;
        }
        Ok(())
    }

    fn visit(&self, visitor: &mut dyn MessageVisitor) {
        visitor.node_id_handshake(self)
    }

    fn clone_box(&self) -> Box<dyn Message> {
        Box::new(self.clone())
    }

    fn message_type(&self) -> MessageType {
        MessageType::NodeIdHandshake
    }
}
