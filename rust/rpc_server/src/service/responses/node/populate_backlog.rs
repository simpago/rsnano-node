use rsnano_node::node::Node;
use rsnano_rpc_messages::SuccessDto;
use serde_json::to_string_pretty;
use std::sync::Arc;

pub async fn populate_backlog(node: Arc<Node>) -> String {
    node.backlog_population.trigger();
    to_string_pretty(&SuccessDto::new()).unwrap()
}

#[cfg(test)]
mod tests {
    use serde_json::to_string;
    use test_helpers::{setup_rpc_client_and_server, System};

    #[test]
    fn populate_backlog() {
        let mut system = System::new();
        let node = system.make_node();

        let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), false);

        let result = node
            .tokio
            .block_on(async { rpc_client.populate_backlog().await.unwrap() });

        assert_eq!(to_string(&result).unwrap(), r#"{"success":""}"#.to_string());

        server.abort();
    }
}
