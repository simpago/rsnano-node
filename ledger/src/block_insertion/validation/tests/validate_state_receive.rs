use crate::{block_insertion::validation::tests::BlockValidationTest, BlockStatus};
use rsnano_core::{AccountInfo, Amount, BlockDetails, BlockHash, BlockSideband, Epoch, PendingKey};

#[test]
fn valid_receive_block() {
    let test = BlockValidationTest::for_epoch2_account()
        .with_pending_receive(Amount::raw(10), Epoch::Epoch0)
        .block_to_validate(|chain| chain.new_receive_block().amount_received(10).build());
    let result = test.assert_is_valid();
    let receive = &test.block();
    let old_account_info = test.chain.account_info();

    assert_eq!(result.account, receive.account_field().unwrap(), "account");
    assert_eq!(
        result.set_account_info,
        AccountInfo {
            head: receive.hash(),
            representative: receive.representative_field().unwrap(),
            open_block: old_account_info.open_block,
            balance: receive.balance_field().unwrap(),
            modified: test.now.as_u64(),
            block_count: old_account_info.block_count + 1,
            epoch: old_account_info.epoch,
        }
    );
    assert_eq!(
        result.set_sideband,
        BlockSideband {
            height: old_account_info.block_count + 1,
            timestamp: test.now.as_u64(),
            successor: BlockHash::zero(),
            account: receive.account_field().unwrap(),
            balance: receive.balance_field().unwrap(),
            details: BlockDetails::new(old_account_info.epoch, false, true, false),
            source_epoch: Epoch::Epoch0
        },
        "sideband"
    );
    assert_eq!(
        result.delete_pending,
        Some(PendingKey::new(
            receive.account_field().unwrap(),
            receive.link_field().unwrap().into()
        ))
    );
    assert_eq!(result.insert_pending, None);
}

#[test]
fn fails_with_gap_source_if_send_block_not_found() {
    BlockValidationTest::for_epoch2_account()
        .with_pending_receive(Amount::raw(10), Epoch::Epoch0)
        .block_to_validate(|chain| chain.new_receive_block().amount_received(10).build())
        .source_block_is_missing()
        .assert_validation_fails_with(BlockStatus::GapSource);
}

#[test]
fn fails_with_balance_mismatch_if_amount_is_wrong() {
    BlockValidationTest::for_epoch2_account()
        .with_pending_receive(Amount::raw(10), Epoch::Epoch0)
        .block_to_validate(|chain| chain.new_receive_block().amount_received(99999).build())
        .assert_validation_fails_with(BlockStatus::BalanceMismatch);
}

#[test]
fn fails_with_balance_mismatch_if_no_link_provided() {
    BlockValidationTest::for_epoch2_account()
        .with_pending_receive(Amount::raw(10), Epoch::Epoch0)
        .block_to_validate(|chain| chain.new_receive_block().link(0).build())
        .assert_validation_fails_with(BlockStatus::BalanceMismatch);
}

#[test]
fn fails_with_unreceivable_if_receiving_from_wrong_account() {
    BlockValidationTest::for_epoch2_account()
        .block_to_validate(|chain| chain.new_receive_block().build())
        .assert_validation_fails_with(BlockStatus::Unreceivable);
}
