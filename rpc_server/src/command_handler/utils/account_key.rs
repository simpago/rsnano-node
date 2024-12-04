use rsnano_rpc_messages::{AccountArg, KeyResponse};

pub(crate) fn account_key(args: AccountArg) -> KeyResponse {
    KeyResponse::new(args.account.into())
}
