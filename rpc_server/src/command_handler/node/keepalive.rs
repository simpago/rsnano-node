use crate::command_handler::RpcCommandHandler;
use rsnano_rpc_messages::{HostWithPortArgs, StartedResponse};

impl RpcCommandHandler {
    pub(crate) fn keepalive(&self, args: HostWithPortArgs) -> anyhow::Result<StartedResponse> {
        self.node
            .rep_crawler
            .keepalive_or_connect(args.address, args.port.into());
        Ok(StartedResponse::new(true))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command_handler::{test_rpc_command_requires_control, test_rpc_command_with_node};
    use rsnano_core::utils::Peer;
    use rsnano_node::Node;
    use rsnano_rpc_messages::RpcCommand;
    use std::sync::Arc;

    #[tokio::test]
    async fn keepalive() {
        let node = Arc::new(Node::new_null());
        let keepalive_tracker = node.rep_crawler.track_keepalives();
        let cmd = RpcCommand::keepalive("foobar.com", 123);

        let result: StartedResponse = test_rpc_command_with_node(cmd, node);

        assert_eq!(result, StartedResponse::new(true));

        let keepalives = keepalive_tracker.output();
        assert_eq!(keepalives, [Peer::new("foobar.com", 123)]);
    }

    #[tokio::test]
    async fn keepalive_fails_without_rpc_control_enabled() {
        test_rpc_command_requires_control(RpcCommand::keepalive("foobar.com", 123));
    }
}
