use crate::{block_insertion::validation::tests::BlockValidationTest, BlockStatus};
use rsnano_core::{Amount, BlockDetails, BlockHash, BlockSideband, Epoch, PendingKey};

#[test]
fn valid_legacy_receive_block() {
    let test = BlockValidationTest::for_epoch0_account()
        .with_pending_receive(Amount::raw(10), Epoch::Epoch0)
        .block_to_validate(|chain| chain.new_legacy_receive_block().build());
    let result = test.assert_is_valid();

    assert_eq!(result.account, test.chain.account());
    assert_eq!(
        result.set_sideband,
        BlockSideband {
            height: test.chain.height() + 1,
            timestamp: test.now,
            successor: BlockHash::zero(),
            account: test.chain.account(),
            balance: test.chain.account_info().balance + Amount::raw(10),
            details: BlockDetails::new(Epoch::Epoch0, false, true, false),
            source_epoch: Epoch::Epoch0,
        }
    );
    assert_eq!(
        result.delete_pending,
        Some(PendingKey::new(
            test.chain.account(),
            test.block().source_or_link()
        ))
    );
    assert_eq!(result.insert_pending, None);
}

#[test]
fn fails_with_unreceivable_if_legacy_send_already_received() {
    BlockValidationTest::for_epoch0_account()
        .block_to_validate(|chain| chain.new_legacy_receive_block().build())
        .assert_validation_fails_with(BlockStatus::Unreceivable);
}

#[test]
fn fails_with_gap_source_if_legacy_source_not_found() {
    BlockValidationTest::for_epoch0_account()
        .block_to_validate(|chain| chain.new_legacy_receive_block().build())
        .source_block_is_missing()
        .assert_validation_fails_with(BlockStatus::GapSource);
}

#[test]
fn fails_if_previous_missing() {
    BlockValidationTest::for_epoch0_account()
        .block_to_validate(|chain| chain.new_legacy_receive_block().build())
        .previous_block_is_missing()
        .assert_validation_fails_with(BlockStatus::GapPrevious);
}

#[test]
fn fails_if_previous_missing_and_account_unknown() {
    BlockValidationTest::for_unopened_account()
        .block_to_validate(|chain| {
            chain
                .new_legacy_receive_block()
                .previous(BlockHash::from(123))
                .build()
        })
        .previous_block_is_missing()
        .assert_validation_fails_with(BlockStatus::GapPrevious);
}

#[test]
fn fails_if_legacy_receive_follows_state_block() {
    BlockValidationTest::for_epoch0_account()
        .setup_account(|chain| {
            chain.add_state();
        })
        .with_pending_receive(Amount::raw(10), Epoch::Epoch0)
        .block_to_validate(|chain| chain.new_legacy_receive_block().build())
        .assert_validation_fails_with(BlockStatus::BlockPosition);
}
