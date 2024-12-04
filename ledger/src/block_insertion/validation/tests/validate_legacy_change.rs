use super::BlockValidationTest;
use crate::BlockStatus;
use rsnano_core::{Account, BlockDetails, BlockHash, BlockSideband, Epoch, PublicKey};

#[test]
fn valid_legacy_change_block() {
    let test = BlockValidationTest::for_epoch0_account().block_to_validate(|chain| {
        chain
            .new_legacy_change_block()
            .representative(PublicKey::from(112233))
            .build()
    });

    let instructions = test.assert_is_valid();

    assert_eq!(instructions.account, test.chain.account());
    assert_eq!(
        instructions.set_sideband,
        BlockSideband {
            height: test.chain.height() + 1,
            timestamp: test.seconds_since_epoch,
            successor: BlockHash::zero(),
            account: test.chain.account(),
            balance: test.chain.account_info().balance,
            details: BlockDetails::new(Epoch::Epoch0, false, false, false),
            source_epoch: Epoch::Epoch0,
        }
    );
    assert_eq!(
        instructions.set_account_info.representative,
        PublicKey::from(112233)
    );
    assert_eq!(
        instructions.set_account_info.balance,
        test.chain.account_info().balance
    );
}

#[test]
fn allow_changing_representative_to_zero() {
    let instructions = BlockValidationTest::for_epoch0_account()
        .block_to_validate(|chain| {
            chain
                .new_legacy_change_block()
                .representative(PublicKey::zero())
                .build()
        })
        .assert_is_valid();

    assert_eq!(
        instructions.set_account_info.representative,
        PublicKey::zero()
    );
}

#[test]
fn allow_changing_from_zero_rep_to_real_rep() {
    let instructions = BlockValidationTest::for_epoch0_account()
        .setup_account(|chain| {
            chain.add_legacy_change(Account::zero());
        })
        .block_to_validate(|chain| {
            chain
                .new_legacy_change_block()
                .representative(PublicKey::from(42))
                .build()
        })
        .assert_is_valid();

    assert_eq!(
        instructions.set_account_info.representative,
        PublicKey::from(42)
    );
}

#[test]
fn fails_with_block_position_if_legacy_change_follows_state_block() {
    BlockValidationTest::for_epoch0_account()
        .setup_account(|chain| {
            chain.add_state();
        })
        .block_to_validate(|chain| chain.new_legacy_change_block().build())
        .assert_validation_fails_with(BlockStatus::BlockPosition);
}
