use super::BlockValidationTest;
use crate::BlockStatus;
use rsnano_core::{AccountInfo, Amount, BlockDetails, Epoch};

#[test]
fn updgrade_to_epoch_v1() {
    let instructions = BlockValidationTest::for_epoch0_account()
        .block_to_validate(|chain| chain.new_epoch1_block().build())
        .assert_is_valid();

    assert_eq!(instructions.set_account_info.epoch, Epoch::Epoch1);
    assert_eq!(
        instructions.set_sideband.details,
        BlockDetails::new(Epoch::Epoch1, false, false, true)
    );
    assert_eq!(instructions.set_sideband.source_epoch, Epoch::Epoch0); // source_epoch is not used for epoch blocks
}

#[test]
fn adding_epoch_twice_fails() {
    BlockValidationTest::for_epoch1_account()
        .block_to_validate(|chain| chain.new_epoch1_block().build())
        .assert_validation_fails_with(BlockStatus::BlockPosition);
}

#[test]
fn adding_legacy_change_block_after_epoch1_fails() {
    BlockValidationTest::for_epoch1_account()
        .block_to_validate(|chain| chain.new_legacy_change_block().build())
        .assert_validation_fails_with(BlockStatus::BlockPosition);
}

#[test]
fn can_add_state_blocks_after_epoch1() {
    BlockValidationTest::for_epoch1_account()
        .block_to_validate(|chain| chain.new_state_block().build())
        .assert_is_valid();
}

#[test]
fn epoch_block_with_changed_representative_fails() {
    BlockValidationTest::for_epoch0_account()
        .block_to_validate(|chain| chain.new_epoch1_block().representative(999999).build())
        .assert_validation_fails_with(BlockStatus::RepresentativeMismatch);
}

#[test]
fn cannot_use_legacy_open_block_if_sender_is_on_epoch1() {
    BlockValidationTest::for_unopened_account()
        .with_pending_receive(Amount::raw(10), Epoch::Epoch1)
        .block_to_validate(|chain| chain.new_legacy_open_block().build())
        .assert_validation_fails_with(BlockStatus::Unreceivable);
}

#[test]
fn cannot_use_legacy_receive_block_after_epoch1_upgrade() {
    BlockValidationTest::for_epoch1_account()
        .with_pending_receive(Amount::raw(10), Epoch::Epoch0)
        .block_to_validate(|chain| chain.new_legacy_receive_block().build())
        .assert_validation_fails_with(BlockStatus::BlockPosition);
}

#[test]
fn cannot_use_legacy_receive_block_after_sender_upgraded_to_epoch1() {
    BlockValidationTest::for_epoch0_account()
        .with_pending_receive(Amount::raw(10), Epoch::Epoch1)
        .block_to_validate(|chain| chain.new_legacy_receive_block().build())
        .assert_validation_fails_with(BlockStatus::Unreceivable);
}

#[test]
fn can_add_state_receive_block_after_epoch1() {
    let instructions = BlockValidationTest::for_epoch1_account()
        .with_pending_receive(Amount::raw(10), Epoch::Epoch1)
        .block_to_validate(|chain| chain.new_receive_block().amount_sent(10).build())
        .assert_is_valid();

    assert_eq!(instructions.set_sideband.details.epoch, Epoch::Epoch1);
    assert_eq!(instructions.set_sideband.source_epoch, Epoch::Epoch1);
}

#[test]
fn can_open_account_with_epoch1_block() {
    let test = BlockValidationTest::for_unopened_account()
        .with_pending_receive(Amount::raw(10), Epoch::Epoch1)
        .block_to_validate(|chain| chain.new_epoch1_open_block().build());
    let instructions = test.assert_is_valid();
    let epoch1_open = test.block();

    assert_eq!(
        instructions.set_account_info,
        AccountInfo {
            head: epoch1_open.hash(),
            representative: epoch1_open.representative_field().unwrap(),
            open_block: epoch1_open.hash(),
            balance: Amount::zero(),
            modified: test.now.as_u64(),
            block_count: 1,
            epoch: Epoch::Epoch1
        }
    )
}

#[test]
fn receiving_from_epoch1_sender_upgrades_receiver_to_epoch1() {
    let instructions = BlockValidationTest::for_epoch0_account()
        .with_pending_receive(Amount::raw(10), Epoch::Epoch1)
        .block_to_validate(|chain| chain.new_receive_block().amount_sent(10).build())
        .assert_is_valid();
    assert_eq!(instructions.set_account_info.epoch, Epoch::Epoch1);
    assert_eq!(instructions.set_sideband.details.epoch, Epoch::Epoch1);
}

#[test]
fn epoch_v1_fork() {
    BlockValidationTest::for_epoch0_account()
        .block_to_validate(|chain| chain.new_epoch1_block().build())
        .setup_account(|chain| {
            chain.add_state();
        })
        .assert_validation_fails_with(BlockStatus::Fork);
}
