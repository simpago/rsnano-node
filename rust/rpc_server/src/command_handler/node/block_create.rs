use crate::command_handler::RpcCommandHandler;
use anyhow::bail;
use rsnano_core::{
    Account, Amount, BlockBuilder, BlockDetails, BlockEnum, BlockHash, Epoch, KeyPair, PendingKey,
    PublicKey, RawKey, WorkVersion,
};
use rsnano_node::Node;
use rsnano_rpc_messages::{BlockCreateArgs, BlockCreateDto, BlockTypeDto, WorkVersionDto};
use std::sync::Arc;

impl RpcCommandHandler {
    pub(crate) fn block_create(&self, args: BlockCreateArgs) -> anyhow::Result<BlockCreateDto> {
        let work_version = args.version.unwrap_or(WorkVersionDto::Work1).into();
        let difficulty = args
            .difficulty
            .unwrap_or_else(|| self.node.ledger.constants.work.threshold_base(work_version));

        let wallet = args.wallet;
        let account = args.account;
        let representative = args.representative;
        let destination = args.destination;
        let source = args.source;
        let amount = args.balance;
        let work = args.work;

        let mut previous = args.previous.unwrap_or(BlockHash::zero());
        let mut balance = args.balance.unwrap_or(Amount::zero());
        let mut prv_key = RawKey::default();

        /*if work.is_none() && !node.distributed_work.work_generation_enabled() {
            return to_string_pretty(&ErrorDto::new("Work generation is disabled".to_string()))
                .unwrap();
        }*/

        if let (Some(wallet_id), Some(account)) = (wallet, account) {
            self.node.wallets.fetch(&wallet_id, &account.into())?;
            let tx = self.node.ledger.read_txn();
            previous = self.node.ledger.any().account_head(&tx, &account).unwrap();
            balance = self
                .node
                .ledger
                .any()
                .account_balance(&tx, &account)
                .unwrap();
        }

        if let Some(key) = args.key {
            prv_key = key;
        }

        //if prv_key.is_zero() {
        //return to_string_pretty(&ErrorDto::new("Block create key required".to_string())).unwrap();
        //}

        let pub_key: PublicKey = (&prv_key).try_into().unwrap();
        let pub_key: Account = pub_key.into();

        /*if let Some(account) = account {
            if account != pub_key {
                return RpcDto::Error(BlockDto(ErrorDto2::BlockRootMismatch))
            }
        }*/

        let key_pair: KeyPair = prv_key.into();

        let mut block = match args.block_type {
            BlockTypeDto::State => {
                if !representative.is_none()
                    && (!args.link.unwrap_or_default().is_zero() || args.link.is_some())
                {
                    let builder = BlockBuilder::state();
                    builder
                        .account(pub_key)
                        .previous(previous)
                        .representative(representative.unwrap())
                        .balance(balance)
                        .link(args.link.unwrap_or_default())
                        .sign(&key_pair)
                        .build()
                } else {
                    bail!(Self::BLOCK_ERROR);
                }
            }
            BlockTypeDto::Open => {
                if !representative.is_none() && source.is_some() {
                    let builder = BlockBuilder::legacy_open();
                    builder
                        .account(pub_key)
                        .source(source.unwrap())
                        .representative(representative.unwrap().into())
                        .sign(&key_pair)
                        .build()
                } else {
                    bail!(Self::BLOCK_ERROR);
                }
            }
            BlockTypeDto::Receive => {
                if source.is_some() {
                    let builder = BlockBuilder::legacy_receive();
                    builder
                        .previous(previous)
                        .source(source.unwrap())
                        .sign(&key_pair)
                        .build()
                } else {
                    bail!(Self::BLOCK_ERROR);
                }
            }
            BlockTypeDto::Change => {
                if !representative.is_none() {
                    let builder = BlockBuilder::legacy_change();
                    builder
                        .previous(previous)
                        .representative(representative.unwrap().into())
                        .sign(&key_pair)
                        .build()
                } else {
                    bail!(Self::BLOCK_ERROR);
                }
            }
            BlockTypeDto::Send => {
                if destination.is_some() && !balance.is_zero() && !amount.is_none() {
                    let amount = amount.unwrap();
                    if balance >= amount {
                        let builder = BlockBuilder::legacy_send();
                        builder
                            .previous(previous)
                            .destination(destination.unwrap())
                            .balance(balance - amount)
                            .sign(key_pair)
                            .build()
                    } else {
                        bail!("Insufficient balance")
                    }
                } else {
                    bail!(Self::BLOCK_ERROR);
                }
            }
            BlockTypeDto::Unknown => {
                bail!(Self::BLOCK_ERROR);
            }
        };

        let root = if !previous.is_zero() {
            previous
        } else {
            pub_key.into()
        };

        if work.is_none() {
            let difficulty = if args.difficulty.is_none() {
                difficulty_ledger(self.node.clone(), &block)
            } else {
                difficulty
            };

            let work = match self.node.distributed_work.make_blocking(
                WorkVersion::Work1,
                root.into(),
                difficulty,
                Some(pub_key),
            ) {
                Some(work) => work,
                None => bail!("Insufficient work"),
            };
            block.set_work(work);
        } else {
            block.set_work(work.unwrap().into());
        }

        let hash = block.hash();
        let difficulty = block.work();
        let json_block = block.json_representation();
        Ok(BlockCreateDto::new(hash, difficulty.into(), json_block))
    }
}

pub fn difficulty_ledger(node: Arc<Node>, block: &BlockEnum) -> u64 {
    let mut details = BlockDetails::new(Epoch::Epoch0, false, false, false);
    let mut details_found = false;

    let transaction = node.store.tx_begin_read();

    // Previous block find
    let mut block_previous: Option<BlockEnum> = None;
    let previous = block.previous();
    if !previous.is_zero() {
        block_previous = node.ledger.any().get_block(&transaction, &previous);
    }

    // Send check
    if let Some(_prev_block) = &block_previous {
        let is_send =
            node.ledger.any().block_balance(&transaction, &previous) > block.balance_field();
        details = BlockDetails::new(Epoch::Epoch0, is_send, false, false);
        details_found = true;
    }

    // Epoch check
    if let Some(prev_block) = &block_previous {
        let epoch = prev_block.sideband().unwrap().details.epoch;
        details = BlockDetails::new(epoch, details.is_send, details.is_receive, details.is_epoch);
    }

    // Link check
    if let Some(link) = block.link_field() {
        if !details.is_send {
            if let Some(block_link) = node.ledger.any().get_block(&transaction, &link.into()) {
                let account = block.account_field().unwrap();
                if node
                    .ledger
                    .any()
                    .get_pending(&transaction, &PendingKey::new(account, link.into()))
                    .is_some()
                {
                    let epoch =
                        std::cmp::max(details.epoch, block_link.sideband().unwrap().details.epoch);
                    details = BlockDetails::new(epoch, details.is_send, true, details.is_epoch);
                    details_found = true;
                }
            }
        }
    }

    if details_found {
        node.network_params.work.threshold(&details)
    } else {
        node.network_params
            .work
            .threshold_base(block.work_version())
    }
}
