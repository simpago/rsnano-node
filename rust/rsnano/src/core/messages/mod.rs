use std::any::Any;

mod message_header;
pub use message_header::*;

mod node_id_handshake;
pub use node_id_handshake::*;

mod keepalive;
pub use keepalive::*;

mod publish;
pub use publish::*;

mod confirm_req;
pub use confirm_req::*;

mod confirm_ack;
pub use confirm_ack::*;

mod frontier_req;
pub use frontier_req::*;

mod bulk_pull;
pub use bulk_pull::*;

mod bulk_pull_account;
pub use bulk_pull_account::*;

mod bulk_push;
pub use bulk_push::*;

mod telemetry_req;
pub use telemetry_req::*;

mod telemetry_ack;
pub use telemetry_ack::*;

mod asc_pull_req;
pub use asc_pull_req::*;

mod asc_pull_ack;
pub use asc_pull_ack::*;

use crate::utils::{MemoryStream, Stream};
use anyhow::Result;

pub trait Message: Send {
    fn header(&self) -> &MessageHeader;
    fn set_header(&mut self, header: &MessageHeader);
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn serialize(&self, stream: &mut dyn Stream) -> Result<()>;
    fn visit(&self, visitor: &mut dyn MessageVisitor);
    fn clone_box(&self) -> Box<dyn Message>;
    fn message_type(&self) -> MessageType;
    fn to_bytes(&self) -> Vec<u8> {
        let mut stream = MemoryStream::new();
        self.serialize(&mut stream).unwrap();
        stream.to_vec()
    }
}

pub trait MessageVisitor {
    fn keepalive(&mut self, _message: &Keepalive) {}
    fn publish(&mut self, _message: &Publish) {}
    fn confirm_req(&mut self, _message: &ConfirmReq) {}
    fn confirm_ack(&mut self, _message: &ConfirmAck) {}
    fn bulk_pull(&mut self, _message: &BulkPull) {}
    fn bulk_pull_account(&mut self, _message: &BulkPullAccount) {}
    fn bulk_push(&mut self, _message: &BulkPush) {}
    fn frontier_req(&mut self, _message: &FrontierReq) {}
    fn node_id_handshake(&mut self, _message: &NodeIdHandshake) {}
    fn telemetry_req(&mut self, _message: &TelemetryReq) {}
    fn telemetry_ack(&mut self, _message: &TelemetryAck) {}
    fn asc_pull_req(&mut self, _message: &AscPullReq) {}
    fn asc_pull_ack(&mut self, _message: &AscPullAck) {}
}

pub trait MessageExt {
    fn to_bytes(&self) -> Vec<u8>;
}
