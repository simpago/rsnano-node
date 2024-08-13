use serde::Deserialize;

#[derive(Deserialize)]
#[serde(tag = "action")]
#[serde(rename_all = "snake_case")]
pub enum WalletRpcRequest {
    AccountCreate {
        wallet: String,
        index: Option<u32>,
    },
    #[serde(other)]
    UnknownCommand,
}
