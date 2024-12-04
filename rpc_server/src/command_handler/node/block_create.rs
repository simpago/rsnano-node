use crate::command_handler::RpcCommandHandler;
use anyhow::bail;
use rsnano_core::{
    Account, Amount, Block, BlockDetails, BlockHash, ChangeBlock, Epoch, OpenBlock, PendingKey,
    PrivateKey, PublicKey, ReceiveBlock, Root, SavedBlock, SendBlock, StateBlock,
};
use rsnano_node::Node;
use rsnano_rpc_messages::{BlockCreateArgs, BlockCreateResponse, BlockTypeDto};
use std::sync::Arc;

impl RpcCommandHandler {
    pub(crate) fn block_create(
        &self,
        args: BlockCreateArgs,
    ) -> anyhow::Result<BlockCreateResponse> {
        let difficulty = args
            .difficulty
            .unwrap_or_else(|| self.node.ledger.constants.work.threshold_base().into())
            .inner();

        let wallet_id = args.wallet.unwrap_or_default();
        let account = args.account.unwrap_or_default();
        let representative = PublicKey::from(args.representative.unwrap_or_default());
        let destination = args.destination.unwrap_or_default();
        let source = args.source.unwrap_or_default();
        let amount = args.balance.unwrap_or_default();
        let work: u64 = args.work.unwrap_or(0.into()).into();

        let mut previous = args.previous.unwrap_or(BlockHash::zero());
        let mut balance = args.balance.unwrap_or(Amount::zero());
        let mut prv_key = PrivateKey::zero();

        if work == 0 && !self.node.distributed_work.work_generation_enabled() {
            bail!("Work generation is disabled");
        }

        if !wallet_id.is_zero() && !account.is_zero() {
            self.node.wallets.fetch(&wallet_id, &account.into())?;
            let tx = self.node.ledger.read_txn();
            previous = self
                .node
                .ledger
                .any()
                .account_head(&tx, &account)
                .unwrap_or_default();
            balance = self
                .node
                .ledger
                .any()
                .account_balance(&tx, &account)
                .unwrap_or_default();
        }

        if let Some(key) = args.key {
            prv_key = key.into();
        }

        let link = args.link.unwrap_or_else(|| {
            // Retrieve link from source or destination
            if source.is_zero() {
                destination.into()
            } else {
                source.into()
            }
        });

        // TODO block_response_put_l
        // TODO get_callback_l

        if prv_key.is_zero() {
            bail!("Private key or local wallet and account required");
        }
        let pub_key = prv_key.public_key();
        let account = Account::from(pub_key);
        // Fetching account balance & previous for send blocks (if aren't given directly)
        if args.previous.is_none() && args.balance.is_none() {
            let tx = self.node.ledger.read_txn();
            previous = self
                .node
                .ledger
                .any()
                .account_head(&tx, &account)
                .unwrap_or_default();
            balance = self
                .node
                .ledger
                .any()
                .account_balance(&tx, &account)
                .unwrap_or_default();
        }
        // Double check current balance if previous block is specified
        else if args.previous.is_some()
            && args.balance.is_some()
            && args.block_type == BlockTypeDto::Send
        {
            let tx = self.node.ledger.read_txn();
            if self.node.ledger.any().block_exists(&tx, &previous)
                && self.node.ledger.any().block_balance(&tx, &previous) != Some(balance)
            {
                bail!("Balance mismatch for previous block");
            }
        }

        // Check for incorrect account key
        if args.account.is_some() {
            if account != account {
                bail!("Incorrect key for given account");
            }
        }

        let root: Root;
        let mut block = match args.block_type {
            BlockTypeDto::State => {
                if args.previous.is_some()
                    && !representative.is_zero()
                    && (!link.is_zero() || args.link.is_some())
                {
                    let block = Block::State(StateBlock::new(
                        account,
                        previous,
                        representative,
                        balance,
                        link,
                        &prv_key,
                        work,
                    ));
                    if previous.is_zero() {
                        root = account.into();
                    } else {
                        root = previous.into();
                    }
                    block
                } else {
                    bail!("Previous, representative, final balance and link (source or destination) are required");
                }
            }
            BlockTypeDto::Open => {
                if !representative.is_zero() && !source.is_zero() {
                    let block = Block::LegacyOpen(OpenBlock::new(
                        source,
                        representative,
                        account,
                        &prv_key,
                        work,
                    ));
                    root = account.into();
                    block
                } else {
                    bail!("Representative account and source hash required");
                }
            }
            BlockTypeDto::Receive => {
                if !source.is_zero() && !previous.is_zero() {
                    let block =
                        Block::LegacyReceive(ReceiveBlock::new(previous, source, &prv_key, work));
                    root = previous.into();
                    block
                } else {
                    bail!("Previous hash and source hash required");
                }
            }
            BlockTypeDto::Change => {
                if !representative.is_zero() && !previous.is_zero() {
                    let block = Block::LegacyChange(ChangeBlock::new(
                        previous,
                        representative,
                        &prv_key,
                        work,
                    ));
                    root = previous.into();
                    block
                } else {
                    bail!("Representative account and previous hash required");
                }
            }
            BlockTypeDto::Send => {
                if !destination.is_zero()
                    && !previous.is_zero()
                    && !balance.is_zero()
                    && !amount.is_zero()
                {
                    if balance >= amount {
                        let block = Block::LegacySend(SendBlock::new(
                            &previous,
                            &destination,
                            &(balance - amount),
                            &prv_key,
                            work,
                        ));
                        root = previous.into();
                        block
                    } else {
                        bail!("Insufficient balance")
                    }
                } else {
                    bail!(
                        "Destination account, previous hash, current balance and amount required"
                    );
                }
            }
            BlockTypeDto::Unknown => {
                bail!("Invalid block type");
            }
        };

        if work == 0 {
            // Difficulty calculation
            let difficulty = if args.difficulty.is_none() {
                difficulty_ledger(self.node.clone(), &block)
            } else {
                difficulty
            };

            let work = match self.node.distributed_work.make_blocking(
                root.into(),
                difficulty,
                Some(account),
            ) {
                Some(work) => work,
                None => bail!("Work generation cancellation or failure"),
            };
            block.set_work(work);
        }

        let json_block = block.json_representation();
        Ok(BlockCreateResponse::new(
            block.hash(),
            self.node
                .network_params
                .work
                .difficulty_block(&block)
                .into(),
            json_block,
        ))
    }
}

pub fn difficulty_ledger(node: Arc<Node>, block: &Block) -> u64 {
    let mut details = BlockDetails::new(Epoch::Epoch0, false, false, false);
    let mut details_found = false;
    let tx = node.store.tx_begin_read();

    // Previous block find
    let mut block_previous: Option<SavedBlock> = None;
    let previous = block.previous();
    if !previous.is_zero() {
        block_previous = node.ledger.any().get_block(&tx, &previous);
    }

    // Send check
    if block_previous.is_some() {
        let is_send = node
            .ledger
            .any()
            .block_balance(&tx, &previous)
            .unwrap_or_default()
            > block.balance_field().unwrap();
        details = BlockDetails::new(Epoch::Epoch0, is_send, false, false);
        details_found = true;
    }

    // Epoch check
    if let Some(prev_block) = &block_previous {
        let epoch = prev_block.epoch();
        details = BlockDetails::new(epoch, details.is_send, details.is_receive, details.is_epoch);
    }

    // Link check
    if let Some(link) = block.link_field() {
        if !details.is_send {
            if let Some(block_link) = node.ledger.any().get_block(&tx, &link.into()) {
                let account = block.account_field().unwrap(); // Link is non-zero therefore it's a state block and has an account field;
                if node
                    .ledger
                    .any()
                    .get_pending(&tx, &PendingKey::new(account, link.into()))
                    .is_some()
                {
                    let epoch = std::cmp::max(details.epoch, block_link.epoch());
                    details = BlockDetails::new(epoch, details.is_send, true, details.is_epoch);
                    details_found = true;
                }
            }
        }
    }

    if details_found {
        node.network_params.work.threshold(&details)
    } else {
        node.network_params.work.threshold_base()
    }
}
