use crate::command_handler::RpcCommandHandler;
use anyhow::bail;
use rsnano_core::BlockDetails;
use rsnano_node::wallets::WalletsExt;
use rsnano_rpc_messages::{BlockDto, SendArgs};

impl RpcCommandHandler {
    pub(crate) fn send(&self, args: SendArgs) -> anyhow::Result<BlockDto> {
        let wallet_id = args.wallet;
        let amount = args.amount;
        // Sending 0 amount is invalid with state blocks
        if amount.is_zero() {
            bail!("Invalid amount number");
        }
        let source = args.source;
        let destination = args.destination;
        let work: u64 = args.work.unwrap_or_default().into();
        if work == 0 && !self.node.distributed_work.work_generation_enabled() {
            bail!("Work generation is disabled");
        }

        let tx = self.node.ledger.read_txn();
        let info = self.load_account(&tx, &source)?;
        let balance = info.balance;

        if work > 0 {
            let details = BlockDetails::new(info.epoch, true, false, false);
            if self
                .node
                .network_params
                .work
                .difficulty(&info.head.into(), work)
                < self.node.network_params.work.threshold(&details)
            {
                bail!("Invalid work")
            }
        }

        let generate_work = work == 0; // Disable work generation if "work" option is provided
        let send_id = args.id;

        let block_hash = self.node.wallets.send_sync(
            wallet_id,
            source,
            destination,
            amount,
            work,
            generate_work,
            send_id,
        );

        if block_hash.is_zero() {
            if balance >= amount {
                bail!("Error generating block")
            } else {
                bail!("Insufficient balance")
            }
        }

        Ok(BlockDto::new(block_hash))
    }
}
