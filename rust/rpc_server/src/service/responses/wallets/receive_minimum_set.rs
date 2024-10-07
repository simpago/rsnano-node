use std::sync::Arc;
use rsnano_core::Amount;
use rsnano_node::node::Node;
use rsnano_rpc_messages::{ErrorDto, SuccessDto};
use serde_json::to_string_pretty;

pub async fn receive_minimum_set(node: Arc<Node>, enable_control: bool, amount: Amount) -> String {
    if enable_control {
        *node.config.receive_minimum.lock().unwrap() = amount;
        to_string_pretty(&SuccessDto::new()).unwrap()
    }
    else {
        to_string_pretty(&ErrorDto::new("RPC control is disabled".to_string())).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use crate::service::responses::test_helpers::setup_rpc_client_and_server;
    use rsnano_core::Amount;
    use rsnano_rpc_messages::SuccessDto;
    use test_helpers::System;

    #[test]
    fn receive_minimum_set() {
        let mut system = System::new();
        let node = system.make_node();

        let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), true);

        let result = node
            .tokio
            .block_on(async { rpc_client.receive_minimum_set(Amount::raw(1000000000000000000000000000000)).await.unwrap() });

        assert_eq!(
            result,
            SuccessDto::new()
        );

        assert_eq!(*node.config.receive_minimum.lock().unwrap(), Amount::raw(1000000000000000000000000000000));

        server.abort();
    }

    #[test]
    fn receive_minimum_set_fails_without_enable_control() {
        let mut system = System::new();
        let node = system.make_node();

        let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), false);

        let result = node
            .tokio
            .block_on(async { rpc_client.receive_minimum_set(Amount::raw(1000000000000000000000000000000)).await });

        assert_eq!(
            result.err().map(|e| e.to_string()),
            Some("node returned error: \"RPC control is disabled\"".to_string())
        );
    
        server.abort();
    }
}