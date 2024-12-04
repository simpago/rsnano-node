use crate::command_handler::RpcCommandHandler;
use rsnano_rpc_messages::UptimeResponse;

impl RpcCommandHandler {
    pub(crate) fn uptime(&self) -> UptimeResponse {
        let seconds = self.node.telemetry.startup_time.elapsed();
        UptimeResponse::new(seconds.as_secs())
    }
}
