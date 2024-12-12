use crate::command_handler::RpcCommandHandler;
use rsnano_core::{Account, Amount, BlockHash, PendingKey};
use rsnano_rpc_messages::{unwrap_u64_or_max, UnopenedArgs, UnopenedResponse};
use std::collections::HashMap;

impl RpcCommandHandler {
    pub(crate) fn unopened(&self, args: UnopenedArgs) -> UnopenedResponse {
        let count = unwrap_u64_or_max(args.count) as usize;
        let threshold = args.threshold.unwrap_or_default();
        let start = args.account.unwrap_or(Account::from(1)); // exclude burn account by default
        let mut accounts: HashMap<Account, Amount> = HashMap::new();

        let tx = self.node.store.tx_begin_read();

        let mut iterator = self
            .node
            .store
            .pending
            .iter_range(&tx, PendingKey::new(start, BlockHash::zero())..);

        let mut current_account = start;
        let mut current_account_sum = Amount::zero();

        let mut current = iterator.next();
        while let Some(cur) = current {
            if accounts.len() >= count {
                break;
            }

            let (key, info) = cur;
            let account = key.receiving_account;

            if self.node.store.account.get(&tx, &account).is_some() {
                if account == Account::MAX {
                    break;
                }
                // Skip existing accounts
                iterator = self.node.store.pending.iter_range(
                    &tx,
                    PendingKey::new(account.inc().unwrap(), BlockHash::zero())..,
                );
                current = iterator.next();
            } else {
                if account != current_account {
                    if !current_account_sum.is_zero() {
                        if current_account_sum >= threshold {
                            accounts.insert(current_account, current_account_sum);
                        }
                        current_account_sum = Amount::zero();
                    }
                    current_account = account;
                }
                current_account_sum += info.amount;
                iterator.next();
                current = iterator.next();
            }
        }

        // last one after iterator reaches end
        if accounts.len() < count
            && !current_account_sum.is_zero()
            && current_account_sum >= threshold
        {
            accounts.insert(current_account, current_account_sum);
        }

        UnopenedResponse::new(accounts)
    }
}
