use crate::command_handler::RpcCommandHandler;
use indexmap::IndexMap;
use rsnano_core::{Account, Amount, BlockHash, PendingInfo, PendingKey};
use rsnano_rpc_messages::{
    unwrap_bool_or_false, unwrap_bool_or_true, unwrap_u64_or_max, AccountsReceivableArgs,
    AccountsReceivableResponse, AccountsReceivableSimple, AccountsReceivableSource,
    AccountsReceivableThreshold, SourceInfo,
};
use std::ops::{Deref, DerefMut};

impl RpcCommandHandler {
    pub(crate) fn accounts_receivable(
        &self,
        args: AccountsReceivableArgs,
    ) -> AccountsReceivableResponse {
        let count = unwrap_u64_or_max(args.count);
        let threshold = args.threshold.unwrap_or(Amount::zero());
        let source = unwrap_bool_or_false(args.source);
        let include_only_confirmed = unwrap_bool_or_true(args.include_only_confirmed);
        let sorting = unwrap_bool_or_false(args.sorting);
        let simple = threshold.is_zero() && !source && !sorting; // if simple, response is a list of hashes for each account
        let tx = self.node.store.tx_begin_read();

        let mut response_builder = if simple {
            ResponseBuilderEnum::Simple(SimpleBuilder::new())
        } else if source {
            ResponseBuilderEnum::Source(SourceBuilder::new())
        } else {
            ResponseBuilderEnum::Threshold(ThresholdBuilder::new())
        };

        for account in args.accounts {
            for (key, info) in self.node.ledger.any().account_receivable_upper_bound(
                &tx,
                account,
                BlockHash::zero(),
            ) {
                if response_builder.len() as u64 >= count {
                    break;
                }

                if include_only_confirmed
                    && !self
                        .node
                        .ledger
                        .confirmed()
                        .block_exists_or_pruned(&tx, &key.send_block_hash)
                {
                    continue;
                }

                if info.amount < threshold {
                    continue;
                }

                response_builder.add(account, &key, &info);
            }
        }

        if sorting {
            response_builder.sort();
        }

        response_builder.finish()
    }
}

enum ResponseBuilderEnum {
    Simple(SimpleBuilder),
    Threshold(ThresholdBuilder),
    Source(SourceBuilder),
}

impl ResponseBuilderEnum {
    fn finish(self) -> AccountsReceivableResponse {
        match self {
            ResponseBuilderEnum::Simple(i) => i.finish(),
            ResponseBuilderEnum::Threshold(i) => i.finish(),
            ResponseBuilderEnum::Source(i) => i.finish(),
        }
    }
}

impl Deref for ResponseBuilderEnum {
    type Target = dyn ResponseBuilder;

    fn deref(&self) -> &Self::Target {
        match self {
            ResponseBuilderEnum::Simple(i) => i,
            ResponseBuilderEnum::Threshold(i) => i,
            ResponseBuilderEnum::Source(i) => i,
        }
    }
}

impl DerefMut for ResponseBuilderEnum {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            ResponseBuilderEnum::Simple(i) => i,
            ResponseBuilderEnum::Threshold(i) => i,
            ResponseBuilderEnum::Source(i) => i,
        }
    }
}

trait ResponseBuilder {
    fn len(&self) -> usize;
    fn add(&mut self, account: Account, key: &PendingKey, info: &PendingInfo);
    fn sort(&mut self);
    fn finish(self) -> AccountsReceivableResponse;
}

struct SimpleBuilder {
    result: IndexMap<Account, Vec<BlockHash>>,
}

impl SimpleBuilder {
    fn new() -> Self {
        Self {
            result: IndexMap::new(),
        }
    }
}

impl ResponseBuilder for SimpleBuilder {
    fn len(&self) -> usize {
        self.result.len()
    }

    fn add(&mut self, account: Account, key: &PendingKey, _info: &PendingInfo) {
        self.result
            .entry(account)
            .or_default()
            .push(key.send_block_hash)
    }

    fn sort(&mut self) {}

    fn finish(self) -> AccountsReceivableResponse {
        AccountsReceivableResponse::Simple(AccountsReceivableSimple {
            blocks: self.result,
        })
    }
}

struct ThresholdBuilder {
    result: IndexMap<Account, IndexMap<BlockHash, Amount>>,
}

impl ThresholdBuilder {
    fn new() -> Self {
        Self {
            result: IndexMap::new(),
        }
    }
}

impl ResponseBuilder for ThresholdBuilder {
    fn len(&self) -> usize {
        self.result.len()
    }

    fn add(&mut self, account: Account, key: &PendingKey, info: &PendingInfo) {
        self.result
            .entry(account)
            .or_default()
            .insert(key.send_block_hash, info.amount);
    }

    fn sort(&mut self) {
        for entry in self.result.values_mut() {
            entry.sort_by(|_, v1, _, v2| v2.cmp(v1));
        }
    }

    fn finish(self) -> AccountsReceivableResponse {
        AccountsReceivableResponse::Threshold(AccountsReceivableThreshold {
            blocks: self.result,
        })
    }
}

struct SourceBuilder {
    result: IndexMap<Account, IndexMap<BlockHash, SourceInfo>>,
}

impl SourceBuilder {
    fn new() -> Self {
        Self {
            result: IndexMap::new(),
        }
    }
}

impl ResponseBuilder for SourceBuilder {
    fn len(&self) -> usize {
        self.result.len()
    }

    fn add(&mut self, account: Account, key: &PendingKey, info: &PendingInfo) {
        self.result.entry(account).or_default().insert(
            key.send_block_hash,
            SourceInfo {
                amount: info.amount,
                source: Some(info.source),
                min_version: None,
            },
        );
    }

    fn sort(&mut self) {
        for entry in self.result.values_mut() {
            entry.sort_by(|_, v1, _, v2| v2.amount.cmp(&v1.amount));
        }
    }

    fn finish(self) -> AccountsReceivableResponse {
        AccountsReceivableResponse::Source(AccountsReceivableSource {
            blocks: self.result,
        })
    }
}
