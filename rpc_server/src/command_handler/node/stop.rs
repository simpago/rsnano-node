use crate::command_handler::RpcCommandHandler;
use rsnano_rpc_messages::SuccessResponse;

impl RpcCommandHandler {
    pub(crate) fn stop(&self) -> SuccessResponse {
        if let Some(tx_stop) = self.stop.lock().unwrap().take() {
            let _ = tx_stop.send(());
        }
        SuccessResponse::new()
    }
}
