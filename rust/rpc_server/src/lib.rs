mod config;
mod service;

pub use config::*;
pub use service::*;

#[cfg(test)]
mod test_helpers {
    use crate::{run_rpc_server, RpcServerConfig};
    use rsnano_core::{utils::get_cpu_count, Networks};
    use rsnano_node::node::Node;
    use rsnano_rpc_client::{NanoRpcClient, Url};
    use std::{
        net::{IpAddr, SocketAddr},
        str::FromStr,
        sync::Arc,
    };
    use test_helpers::get_available_port;

    fn setup_rpc_client_and_server(
        node: Arc<Node>,
    ) -> (
        Arc<NanoRpcClient>,
        tokio::task::JoinHandle<Result<(), anyhow::Error>>,
    ) {
        let port = get_available_port();
        let rpc_server_config =
            RpcServerConfig::default_for(Networks::NanoBetaNetwork, get_cpu_count());
        let ip_addr = IpAddr::from_str(&rpc_server_config.address).unwrap();
        let socket_addr = SocketAddr::new(ip_addr, port);

        let server = node
            .tokio
            .spawn(run_rpc_server(node.clone(), socket_addr, true));

        let rpc_url = format!("http://[::1]:{}/", port);
        let rpc_client = Arc::new(NanoRpcClient::new(Url::parse(&rpc_url).unwrap()));

        (rpc_client, server)
    }
}
