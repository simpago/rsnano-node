use rsnano_core::{Account, Amount};
use rsnano_node::node::Node;
use rsnano_rpc_messages::{AccountsWithAmountsDto, ErrorDto};
use serde_json::to_string_pretty;
use std::{collections::HashMap, sync::Arc};

pub async fn unopened(
    node: Arc<Node>,
    enable_control: bool,
    account: Account,
    count: u64,
    threshold: Option<Amount>,
) -> String {
    if !enable_control {
        return to_string_pretty(&ErrorDto::new("RPC control is disabled".to_string())).unwrap();
    }

    let start = account;
    let mut accounts: HashMap<Account, Amount> = HashMap::new();

    let transaction = node.store.tx_begin_read();
    let mut iterator = node.store.pending.begin(&transaction);
    let end = node.store.pending.end();

    let mut current_account = start;
    let mut current_account_sum = Amount::zero();

    while iterator != end && accounts.len() < count as usize {
        let (key, info) = iterator.current().unwrap();
        let account = key.receiving_account;

        if node.store.account.get(&transaction, &account).is_some() {
            iterator = node.store.pending.begin_at_key(&transaction, key);
        } else {
            if account != current_account {
                if !current_account_sum.is_zero() {
                    if threshold.map_or(true, |t| current_account_sum >= t) {
                        accounts.insert(current_account, current_account_sum);
                    }
                    current_account_sum = Amount::zero();
                }
                current_account = account;
            }
            current_account_sum += info.amount;
        }
        iterator.next();
    }

    if accounts.len() < count as usize
        && !current_account_sum.is_zero()
        && threshold.map_or(true, |t| current_account_sum >= t)
    {
        accounts.insert(current_account, current_account_sum);
    }

    let response = AccountsWithAmountsDto::new("accounts".to_string(), accounts);

    to_string_pretty(&response).unwrap()
}

#[cfg(test)]
mod tests {
    use rsnano_core::{Account, Amount, BlockEnum, StateBlock, DEV_GENESIS_KEY};
    use rsnano_ledger::{DEV_GENESIS_ACCOUNT, DEV_GENESIS_HASH, DEV_GENESIS_PUB_KEY};
    use rsnano_node::node::Node;
    use std::sync::Arc;
    use std::time::Duration;
    use test_helpers::{assert_timely_msg, setup_rpc_client_and_server, System};

    fn send_block(node: Arc<Node>) {
        let send1 = BlockEnum::State(StateBlock::new(
            *DEV_GENESIS_ACCOUNT,
            *DEV_GENESIS_HASH,
            *DEV_GENESIS_PUB_KEY,
            Amount::MAX - Amount::raw(1),
            Account::zero().into(),
            &DEV_GENESIS_KEY,
            node.work_generate_dev((*DEV_GENESIS_HASH).into()),
        ));

        node.process_active(send1.clone());
        assert_timely_msg(
            Duration::from_secs(5),
            || node.active.active(&send1),
            "not active on node 1",
        );
    }

    #[test]
    fn unopened() {
        let mut system = System::new();
        let node = system.make_node();

        send_block(node.clone());

        let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), true);

        let result = node
            .tokio
            .block_on(async { rpc_client.unopened(Account::zero(), 1, None).await.unwrap() });

        assert_eq!(result.value.get(&Account::zero()).unwrap(), &Amount::raw(1));

        server.abort();
    }

    #[test]
    fn unopened_fails_without_enable_control() {
        let mut system = System::new();
        let node = system.make_node();

        let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), false);

        let result = node
            .tokio
            .block_on(async { rpc_client.unopened(Account::zero(), 1, None).await });

        assert_eq!(
            result.err().map(|e| e.to_string()),
            Some("node returned error: \"RPC control is disabled\"".to_string())
        );

        server.abort();
    }
}
