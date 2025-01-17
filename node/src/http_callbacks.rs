use crate::{
    consensus::{ElectionStatus, ElectionStatusType},
    stats::{DetailType, Direction, StatType, Stats},
};
use rsnano_core::{Account, Amount, BlockType, SavedBlock};
use rsnano_nullable_http_client::{HttpClient, Url};
use serde::Serialize;
use std::sync::Arc;
use tracing::error;

/// Performs an HTTP callback to a configured endpoint
/// if a block is confirmed
pub(crate) struct HttpCallbacks {
    pub runtime: tokio::runtime::Handle,
    pub stats: Arc<Stats>,
    pub callback_url: Url,
}

impl HttpCallbacks {
    pub fn execute(
        &self,
        status: &ElectionStatus,
        account: Account,
        block: &SavedBlock,
        amount: Amount,
        is_state_send: bool,
        is_state_epoch: bool,
    ) {
        let block = block.clone();
        if status.election_status_type == ElectionStatusType::ActiveConfirmedQuorum
            || status.election_status_type == ElectionStatusType::ActiveConfirmationHeight
        {
            let url = self.callback_url.clone();
            let stats = self.stats.clone();
            self.runtime.spawn(async move {
                let message = RpcCallbackMessage {
                    account: account.encode_account(),
                    hash: block.hash().encode_hex(),
                    block: (*block).clone().into(),
                    amount: amount.to_string_dec(),
                    sub_type: if is_state_send {
                        Some("send")
                    } else if block.block_type() == BlockType::State {
                        if block.is_change() {
                            Some("change")
                        } else if is_state_epoch {
                            Some("epoch")
                        } else {
                            Some("receive")
                        }
                    } else {
                        None
                    },
                    is_send: if is_state_send { Some("true") } else { None },
                };

                let http_client = HttpClient::new();
                match http_client.post_json(url.clone(), &message).await {
                    Ok(response) => {
                        if response.status().is_success() {
                            stats.inc_dir(
                                StatType::HttpCallbacks,
                                DetailType::Initiate,
                                Direction::Out,
                            );
                        } else {
                            error!(
                                "Callback to {} failed [status: {:?}]",
                                url,
                                response.status()
                            );
                            stats.inc_dir(
                                StatType::Error,
                                DetailType::HttpCallback,
                                Direction::Out,
                            );
                        }
                    }
                    Err(e) => {
                        error!("Unable to send callback: {} ({})", url, e);
                        stats.inc_dir(StatType::Error, DetailType::HttpCallback, Direction::Out);
                    }
                }
            });
        }
    }
}

#[derive(Serialize)]
struct RpcCallbackMessage {
    account: String,
    hash: String,
    block: serde_json::Value,
    amount: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    sub_type: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    is_send: Option<&'static str>,
}
