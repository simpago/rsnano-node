use rsnano_node::node::Node;
use rsnano_rpc_messages::ConfirmationActiveDto;
use serde_json::to_string_pretty;
use std::{sync::Arc, usize};

pub async fn confirmation_active(node: Arc<Node>, announcements: Option<u64>) -> String {
    let announcements = announcements.unwrap_or(0);
    let mut confirmed = 0;
    let mut confirmations = Vec::new();

    let active_elections = node.active.list_active(usize::MAX);
    for election in active_elections {
        if election
            .confirmation_request_count
            .load(std::sync::atomic::Ordering::Relaxed) as u64
            >= announcements
        {
            if !node.active.confirmed(&election) {
                confirmations.push(election.qualified_root.clone());
            } else {
                confirmed += 1;
            }
        }
    }

    let unconfirmed = confirmations.len() as u64;

    let confirmation_active_dto = ConfirmationActiveDto::new(confirmations, unconfirmed, confirmed);

    to_string_pretty(&confirmation_active_dto).unwrap()
}

#[cfg(test)]
mod tests {
    use test_helpers::{send_block, setup_rpc_client_and_server, System};

    #[test]
    fn confirmation_active() {
        let mut system = System::new();
        let node = system.make_node();

        let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), false);

        send_block(node.clone());

        let result = node
            .tokio
            .block_on(async { rpc_client.confirmation_active(None).await.unwrap() });

        assert!(!result.confirmations.is_empty());
        assert_eq!(result.confirmed, 0);
        assert_eq!(result.unconfirmed, 1);

        server.abort();
    }
}
