use super::BlockValidationTest;
use crate::BlockStatus;
use rsnano_core::{epoch_v2_link, Amount, Epoch, PrivateKey};

#[test]
fn fails_if_directly_upgrading_from_epoch_0_to_epoch_2() {
    // Trying to upgrade from epoch 0 to epoch 2.
    // It is a requirement epoch upgrades are sequential unless the account is unopened
    BlockValidationTest::for_epoch0_account()
        .block_to_validate(|chain| chain.new_epoch2_block().build())
        .assert_validation_fails_with(BlockStatus::BlockPosition);
}

#[test]
fn upgrade_from_epoch_1_to_epoch_2() {
    let instructions = BlockValidationTest::for_epoch1_account()
        .block_to_validate(|chain| chain.new_epoch2_block().build())
        .assert_is_valid();
    assert_eq!(instructions.set_account_info.epoch, Epoch::Epoch2);
    assert_eq!(instructions.set_sideband.details.epoch, Epoch::Epoch2);
}

#[test]
fn upgrading_to_epoch_v2_twice_fails() {
    BlockValidationTest::for_epoch2_account()
        .block_to_validate(|chain| chain.new_epoch2_block().build())
        .assert_validation_fails_with(BlockStatus::BlockPosition);
}

#[test]
fn legacy_receive_block_after_epoch_v2_upgrade_fails() {
    BlockValidationTest::for_epoch2_account()
        .with_pending_receive(Amount::raw(10), Epoch::Epoch0)
        .block_to_validate(|chain| chain.new_legacy_receive_block().build())
        .assert_validation_fails_with(BlockStatus::BlockPosition);
}

#[test]
fn cannot_use_legacy_open_block_with_epoch_v2_send() {
    BlockValidationTest::for_unopened_account()
        .with_pending_receive(Amount::raw(10), Epoch::Epoch2)
        .block_to_validate(|chain| chain.new_legacy_open_block().build())
        .assert_validation_fails_with(BlockStatus::Unreceivable);
}

#[test]
fn receive_after_epoch_v2_upgrade() {
    BlockValidationTest::for_epoch2_account()
        .with_pending_receive(Amount::raw(10), Epoch::Epoch2)
        .block_to_validate(|chain| chain.new_receive_block().amount_sent(10).build())
        .assert_is_valid();
}

#[test]
fn receiving_from_epoch_2_block_upgrades_receiver_to_epoch2() {
    let instructions = BlockValidationTest::for_epoch0_account()
        .with_pending_receive(Amount::raw(10), Epoch::Epoch2)
        .block_to_validate(|chain| chain.new_receive_block().amount_sent(10).build())
        .assert_is_valid();
    assert_eq!(instructions.set_account_info.epoch, Epoch::Epoch2);
    assert_eq!(instructions.set_sideband.details.epoch, Epoch::Epoch2);
}

#[test]
fn open_new_account_straight_to_epoch_2() {
    let instructions = BlockValidationTest::for_unopened_account()
        .with_pending_receive(Amount::raw(10), Epoch::Epoch2)
        .block_to_validate(|chain| chain.new_open_block().balance(10).build())
        .assert_is_valid();
    assert_eq!(instructions.set_account_info.epoch, Epoch::Epoch2);
    assert_eq!(instructions.set_sideband.details.epoch, Epoch::Epoch2);
}

#[test]
fn fails_with_bad_signature_if_signature_is_invalid() {
    BlockValidationTest::for_epoch1_account()
        .block_to_validate(|chain| {
            chain
                .new_epoch1_block()
                .link(epoch_v2_link())
                .sign(&PrivateKey::new())
                .build()
        })
        .assert_validation_fails_with(BlockStatus::BadSignature);
}
