use rsnano_core::Amount;
use rsnano_rpc_messages::AmountRpcMessage;

pub fn raw_to_nano(args: AmountRpcMessage) -> AmountRpcMessage {
    AmountRpcMessage::new(Amount::raw(args.amount.number() / Amount::nano(1).number()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_raw_to_nano() {
        assert_eq!(
            raw_to_nano(AmountRpcMessage::new(Amount::raw(
                12_400_000_000_000_000_000_000_000_000_000
            )))
            .amount
            .number(),
            12
        );
    }
}
