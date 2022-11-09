use super::MessageHandle;
use crate::{
    core::messages::{
        BulkPull, BulkPullAccount, BulkPush, ConfirmAck, ConfirmReq, FrontierReq, Keepalive,
        Message, MessageVisitor, NodeIdHandshake, Publish, TelemetryAck, TelemetryReq,
    },
    ffi::VoidPointerCallback,
    transport::BootstrapMessageVisitor,
};
use std::ffi::c_void;

type MessageVisitorCallback = unsafe extern "C" fn(*mut c_void, *mut MessageHandle, u8);
static mut MESSAGE_VISITOR_VISIT: Option<MessageVisitorCallback> = None;
static mut MESSAGE_VISITOR_DESTROY: Option<VoidPointerCallback> = None;

#[no_mangle]
pub unsafe extern "C" fn rsn_callback_message_visitor_visit(f: MessageVisitorCallback) {
    MESSAGE_VISITOR_VISIT = Some(f);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_callback_message_visitor_destroy(f: VoidPointerCallback) {
    MESSAGE_VISITOR_DESTROY = Some(f);
}

pub(crate) struct FfiMessageVisitor {
    /// a `shared_ptr<message_visitor> *`
    handle: *mut c_void,
}

impl FfiMessageVisitor {
    pub(crate) fn new(handle: *mut c_void) -> Self {
        Self { handle }
    }

    fn visit_callback<T>(&self, message: &T)
    where
        T: 'static + Message + Clone,
    {
        let message_handle = MessageHandle::from_message(message.clone());
        unsafe {
            match MESSAGE_VISITOR_VISIT {
                Some(f) => f(
                    self.handle,
                    message_handle,
                    message.header().message_type() as u8,
                ),
                None => panic!("MESSAGE_VISITOR_CALLBACK missing"),
            }
        }
    }
}

impl MessageVisitor for FfiMessageVisitor {
    fn keepalive(&mut self, message: &Keepalive) {
        self.visit_callback(message);
    }

    fn publish(&mut self, message: &Publish) {
        self.visit_callback(message);
    }

    fn confirm_req(&mut self, message: &ConfirmReq) {
        self.visit_callback(message);
    }

    fn confirm_ack(&mut self, message: &ConfirmAck) {
        self.visit_callback(message);
    }

    fn bulk_pull(&mut self, message: &BulkPull) {
        self.visit_callback(message);
    }

    fn bulk_pull_account(&mut self, message: &BulkPullAccount) {
        self.visit_callback(message);
    }

    fn bulk_push(&mut self, message: &BulkPush) {
        self.visit_callback(message);
    }

    fn frontier_req(&mut self, message: &FrontierReq) {
        self.visit_callback(message);
    }

    fn node_id_handshake(&mut self, message: &NodeIdHandshake) {
        self.visit_callback(message);
    }

    fn telemetry_req(&mut self, message: &TelemetryReq) {
        self.visit_callback(message);
    }

    fn telemetry_ack(&mut self, message: &TelemetryAck) {
        self.visit_callback(message);
    }
}

pub type MessageVisitorFlagCallback = unsafe extern "C" fn(*mut c_void) -> bool;
static mut BOOTSTRAP_PROCESSED: Option<MessageVisitorFlagCallback> = None;

#[no_mangle]
pub unsafe extern "C" fn rsn_callback_message_visitor_bootstrap_processed(
    f: MessageVisitorFlagCallback,
) {
    BOOTSTRAP_PROCESSED = Some(f);
}

impl BootstrapMessageVisitor for FfiMessageVisitor {
    fn processed(&self) -> bool {
        unsafe { BOOTSTRAP_PROCESSED.expect("BOOTSTRAP_PROCESSED missing")(self.handle) }
    }

    fn as_message_visitor(&mut self) -> &mut dyn MessageVisitor {
        self
    }
}

impl Drop for FfiMessageVisitor {
    fn drop(&mut self) {
        unsafe {
            match MESSAGE_VISITOR_DESTROY {
                Some(f) => f(self.handle),
                None => panic!("MESSAGE_VISITOR_DESTROY missing"),
            }
        }
    }
}
