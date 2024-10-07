use rsnano_core::{
    AccountInfo, Amount, BlockDetails, BlockHash, BlockSideband, Epoch, PendingInfo, PendingKey,
};

use crate::{block_insertion::validation::tests::BlockValidationTest, BlockStatus};

#[test]
fn valid_legacy_send_block() {
    let test = BlockValidationTest::for_epoch0_account()
        .block_to_validate(|chain| chain.new_legacy_send_block().amount(1).build());
    let result = test.assert_is_valid();

    assert_eq!(result.account, test.chain.account());
    assert_eq!(
        result.set_sideband,
        BlockSideband {
            height: test.chain.height() + 1,
            timestamp: test.seconds_since_epoch,
            successor: BlockHash::zero(),
            account: test.chain.account(),
            balance: test.chain.account_info().balance - Amount::raw(1),
            details: BlockDetails::new(Epoch::Epoch0, true, false, false),
            source_epoch: Epoch::Epoch0
        }
    );
    assert_eq!(result.delete_pending, None);
    assert_eq!(
        result.insert_pending,
        Some((
            PendingKey::new(test.block().destination_or_link(), test.block().hash()),
            PendingInfo {
                source: test.chain.account(),
                amount: Amount::raw(1),
                epoch: Epoch::Epoch0,
            }
        ))
    );
    assert_eq!(
        result.set_account_info,
        AccountInfo {
            head: test.block().hash(),
            representative: test.chain.account_info().representative,
            open_block: test.chain.open(),
            balance: test.chain.account_info().balance - Amount::raw(1),
            modified: test.seconds_since_epoch,
            block_count: test.chain.height() + 1,
            epoch: Epoch::Epoch0
        }
    );
}

#[test]
fn fails_with_old_if_legacy_sending_twice() {
    BlockValidationTest::for_epoch0_account()
        .block_to_validate(|chain| chain.new_legacy_send_block().build())
        .block_already_exists()
        .assert_validation_fails_with(BlockStatus::Old);
}

#[test]
fn fails_with_fork_if_legacy_send_block_has_unexpected_previous_block() {
    BlockValidationTest::for_epoch0_account()
        .block_to_validate(|chain| {
            chain
                .new_legacy_send_block()
                .previous(BlockHash::from(99999))
                .build()
        })
        .assert_validation_fails_with(BlockStatus::Fork);
}

#[test]
fn fails_if_sending_negative_amount() {
    BlockValidationTest::for_epoch0_account()
        .block_to_validate(|chain| {
            chain
                .new_legacy_send_block()
                .balance(Amount::nano(9999))
                .build()
        })
        .assert_validation_fails_with(BlockStatus::NegativeSpend);
}

#[test]
fn send_fails_if_work_is_insufficient_for_epoch_0() {
    BlockValidationTest::for_epoch0_account()
        .block_to_validate(|chain| chain.new_legacy_send_block().work(0).build())
        .assert_validation_fails_with(BlockStatus::InsufficientWork);
}

#[test]
fn fails_if_legacy_send_follows_a_state_block() {
    BlockValidationTest::for_epoch0_account()
        .setup_account(|chain| {
            chain.add_state();
        })
        .block_to_validate(|chain| chain.new_legacy_send_block().build())
        .assert_validation_fails_with(BlockStatus::BlockPosition);
}

#[test]
fn when_pending_receive_exists_for_link_dont_delete_it() {
    let test = BlockValidationTest::for_epoch0_account()
        .block_to_validate(|chain| chain.new_legacy_send_block().amount(1).build())
        .with_pending_receive(Amount::raw(1), Epoch::Epoch0);

    let result = test.assert_is_valid();

    assert_eq!(result.delete_pending, None);
}
