use rsnano_rpc_messages::{AccountResponse, KeyArg};

pub(crate) fn account_get(args: KeyArg) -> AccountResponse {
    AccountResponse::new(args.key.into())
}
