use crate::command_handler::RpcCommandHandler;
use rsnano_core::{Account, Amount};
use rsnano_rpc_messages::AvailableSupplyReponse;

impl RpcCommandHandler {
    pub(crate) fn available_supply(&self) -> AvailableSupplyReponse {
        let tx = self.node.store.env.tx_begin_read();
        // Cold storage genesis
        let genesis_balance = self
            .node
            .balance(&self.node.network_params.ledger.genesis_account);

        // Active unavailable account
        let landing_balance = self.node.balance(
            &Account::decode_hex(
                "059F68AAB29DE0D3A27443625C7EA9CDDB6517A8B76FE37727EF6A4D76832AD5",
            )
            .unwrap(),
        );

        // Faucet account
        let faucet_balance = self.node.balance(
            &Account::decode_hex(
                "8E319CE6F3025E5B2DF66DA7AB1467FE48F1679C13DD43BFDB29FA2E9FC40D3B",
            )
            .unwrap(),
        );

        // Burning 0 account
        let burned_balance = self.node.ledger.account_receivable(
            &tx,
            &Account::decode_account(
                "nano_1111111111111111111111111111111111111111111111111111hifc8npp",
            )
            .unwrap(),
            false,
        );

        let available =
            Amount::MAX - genesis_balance - landing_balance - faucet_balance - burned_balance;
        AvailableSupplyReponse::new(available)
    }
}
