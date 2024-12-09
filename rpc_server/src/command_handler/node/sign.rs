use crate::command_handler::RpcCommandHandler;
use anyhow::bail;
use rsnano_core::{Block, PrivateKey};
use rsnano_rpc_messages::{SignArgs, SignResponse};

impl RpcCommandHandler {
    pub(crate) fn sign(&self, args: SignArgs) -> anyhow::Result<SignResponse> {
        // Retrieving hash
        let mut hash = args.hash.unwrap_or_default();
        // Retrieving block
        let block = args.block.map(|b| Block::from(b));
        if let Some(b) = &block {
            hash = b.hash();
        }
        // Hash or block are not initialized
        if hash.is_zero() {
            bail!("Block is invalid")
        }
        // Hash is initialized without config permission
        // TODO Check sign hash pemrmission!

        let prv: PrivateKey = if let Some(key) = args.key {
            // Retrieving private key from request
            key.into()
        } else {
            // Retrieving private key from wallet
            if args.wallet.is_some() && args.account.is_some() {
                self.node
                    .wallets
                    .fetch(&args.wallet.unwrap(), &args.account.unwrap().into())?
                    .into()
            } else {
                PrivateKey::zero()
            }
        };

        // Signing
        if prv.is_zero() {
            bail!("Private key or local wallet and account required");
        }

        let signature = prv.sign(hash.as_bytes());
        let json_block = if let Some(mut block) = block {
            block.set_signature(signature.clone());
            Some(block.json_representation())
        } else {
            None
        };

        Ok(SignResponse {
            signature,
            block: json_block,
        })
    }
}
