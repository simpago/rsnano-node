use crate::command_handler::RpcCommandHandler;
use anyhow::bail;
use rsnano_node::bootstrap::BootstrapInitiatorExt;
use rsnano_rpc_messages::{unwrap_bool_or_false, BootstrapAnyArgs, SuccessResponse};

impl RpcCommandHandler {
    pub(crate) fn bootstrap_any(&self, args: BootstrapAnyArgs) -> anyhow::Result<SuccessResponse> {
        if self.node.flags.disable_legacy_bootstrap {
            bail!("Legacy bootstrap is disabled");
        }

        let force = unwrap_bool_or_false(args.force);
        let bootstrap_id = args.id.unwrap_or_default();
        let start_account = args.account.unwrap_or_default();

        self.node
            .bootstrap_initiator
            .bootstrap(force, bootstrap_id, u32::MAX, start_account);

        Ok(SuccessResponse::new())
    }
}
