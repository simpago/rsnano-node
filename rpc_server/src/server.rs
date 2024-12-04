use crate::command_handler::RpcCommandHandler;
use anyhow::{Context, Result};
use axum::{extract::State, http::Request, middleware::map_request, routing::post, Json, Router};
use rsnano_node::Node;
use rsnano_rpc_messages::RpcCommand;
use std::{future::Future, sync::Arc};
use tokio::{net::TcpListener, task::spawn_blocking};
use tracing::info;

pub async fn run_rpc_server<F>(
    node: Arc<Node>,
    listener: TcpListener,
    enable_control: bool,
    tx_stop: tokio::sync::oneshot::Sender<()>,
    shutdown: F,
) -> Result<()>
where
    F: Future<Output = ()> + Send + 'static,
{
    let command_handler = RpcCommandHandler::new(node, enable_control, tx_stop);

    let app = Router::new()
        .route("/", post(handle_rpc))
        .layer(map_request(set_json_content))
        .with_state(command_handler);

    info!("RPC listening address: {}", listener.local_addr()?);

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown)
        .await
        .context("Failed to run the server")
}

async fn handle_rpc(
    State(command_handler): State<RpcCommandHandler>,
    Json(command): Json<RpcCommand>,
) -> Json<serde_json::Value> {
    let response = spawn_blocking(move || command_handler.handle(command))
        .await
        .unwrap();
    Json(response)
}

/// JSON is the default and the only accepted content type!
async fn set_json_content<B>(mut request: Request<B>) -> Request<B> {
    request
        .headers_mut()
        .insert("Content-Type", "application/json".parse().unwrap());
    request
}
