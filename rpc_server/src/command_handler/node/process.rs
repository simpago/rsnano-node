use crate::command_handler::RpcCommandHandler;
use anyhow::{anyhow, bail};
use rsnano_core::{Block, BlockBase, BlockType};
use rsnano_ledger::BlockStatus;
use rsnano_network::ChannelId;
use rsnano_node::block_processing::BlockSource;
use rsnano_rpc_messages::{BlockSubTypeDto, HashRpcMessage, ProcessArgs, StartedResponse};

impl RpcCommandHandler {
    pub(crate) fn process(&self, args: ProcessArgs) -> anyhow::Result<serde_json::Value> {
        let is_async = args.is_async.unwrap_or_default().inner();
        let block: Block = args.block.into();

        // State blocks subtype check
        if let Block::State(state) = &block {
            if let Some(subtype) = args.subtype {
                let tx = self.node.ledger.read_txn();
                if !state.previous().is_zero()
                    && !self.node.ledger.any().block_exists(&tx, &state.previous())
                {
                    bail!("Gap previous block")
                } else {
                    let balance = self
                        .node
                        .ledger
                        .any()
                        .account_balance(&tx, &state.account())
                        .unwrap_or_default();
                    match subtype {
                        BlockSubTypeDto::Send => {
                            if balance <= state.balance() {
                                bail!("Invalid block balance for given subtype");
                            }
                            // Send with previous == 0 fails balance check. No previous != 0 check required
                        }
                        BlockSubTypeDto::Receive => {
                            if balance > state.balance() {
                                bail!("Invalid block balance for given subtype");
                            }
                            // Receive can be point to open block. No previous != 0 check required
                        }
                        BlockSubTypeDto::Open => {
                            if !state.previous().is_zero() {
                                bail!("Invalid previous block for given subtype");
                            }
                        }
                        BlockSubTypeDto::Change => {
                            if balance != state.balance() {
                                bail!("Invalid block balance for given subtype");
                            } else if state.previous().is_zero() {
                                bail!("Invalid previous block for given subtype");
                            }
                        }
                        BlockSubTypeDto::Epoch => {
                            if balance != state.balance() {
                                bail!("Invalid block balance for given subtype");
                            } else if !self.node.ledger.is_epoch_link(&state.link()) {
                                bail!("Invalid epoch link");
                            }
                        }
                        BlockSubTypeDto::Unknown => bail!("Invalid block subtype"),
                    }
                }
            }
        }

        if !self.node.network_params.work.validate_entry_block(&block) {
            bail!("Block work is less than threshold");
        }

        if !is_async {
            let hash = block.hash();
            let Some(result) = self.node.process_local(block.clone()) else {
                bail!("Stopped");
            };
            match result {
                BlockStatus::Progress => Ok(serde_json::to_value(HashRpcMessage::new(hash))?),
                BlockStatus::GapPrevious => Err(anyhow!("Gap previous block")),
                BlockStatus::BadSignature => Err(anyhow!("Bad signature")),
                BlockStatus::Old => Err(anyhow!("Old block")),
                BlockStatus::NegativeSpend => Err(anyhow!("Negative spend")),
                BlockStatus::Fork => {
                    if args.force.unwrap_or_default().inner() {
                        self.node.active.erase(&block.qualified_root());
                        self.node.block_processor.force(block.into());
                        Ok(serde_json::to_value(HashRpcMessage::new(hash))?)
                    } else {
                        Err(anyhow!("Fork"))
                    }
                }
                BlockStatus::Unreceivable => Err(anyhow!("Unreceivable")),
                BlockStatus::GapSource => Err(anyhow!("Gap source block")),
                BlockStatus::GapEpochOpenPending => {
                    Err(anyhow!("Gap pending for open epoch block"))
                }
                BlockStatus::OpenedBurnAccount => {
                    Err(anyhow!("Block attempts to open the burn account"))
                }
                BlockStatus::BalanceMismatch => {
                    Err(anyhow!("Balance and amount delta do not match"))
                }
                BlockStatus::RepresentativeMismatch => Err(anyhow!("Representative mismatch")),
                BlockStatus::BlockPosition => {
                    Err(anyhow!("This block cannot follow the previous block"))
                }
                BlockStatus::InsufficientWork => Err(anyhow!("Block work is insufficient")),
            }
        } else {
            if block.block_type() == BlockType::State {
                self.node.block_processor.add(
                    block.into(),
                    BlockSource::Local,
                    ChannelId::LOOPBACK,
                );
                Ok(serde_json::to_value(StartedResponse::new(true))?)
            } else {
                Err(anyhow!("Must be a state block"))
            }
        }
    }
}
