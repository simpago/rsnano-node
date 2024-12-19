use rsnano_core::{
    AccountInfo, Amount, BlockDetails, BlockHash, BlockSideband, Epoch, PendingKey, PrivateKey,
};

use crate::{block_insertion::validation::tests::BlockValidationTest, BlockStatus};

#[test]
fn valid_legacy_open_block() {
    let test = BlockValidationTest::for_unopened_account()
        .with_pending_receive(Amount::raw(100), Epoch::Epoch0)
        .block_to_validate(|chain| chain.new_legacy_open_block().build());
    let result = test.assert_is_valid();
    let block = test.block();

    assert_eq!(result.account, test.chain.account());
    assert_eq!(
        result.set_sideband,
        BlockSideband {
            height: 1,
            timestamp: test.now,
            successor: BlockHash::zero(),
            account: test.chain.account(),
            balance: Amount::raw(100),
            details: BlockDetails::new(Epoch::Epoch0, false, true, false),
            source_epoch: Epoch::Epoch0
        }
    );
    assert_eq!(
        result.delete_pending,
        Some(PendingKey::new(
            test.chain.account(),
            block.source_or_link()
        ))
    );
    assert_eq!(result.insert_pending, None);
    assert_eq!(
        result.set_account_info,
        AccountInfo {
            head: block.hash(),
            representative: block.representative_field().unwrap(),
            open_block: block.hash(),
            balance: Amount::raw(100),
            modified: test.now.as_u64(),
            block_count: 1,
            epoch: Epoch::Epoch0
        }
    );
}

#[test]
fn fail_fork() {
    BlockValidationTest::for_epoch0_account()
        .block_to_validate(|chain| chain.new_legacy_open_block().build())
        .assert_validation_fails_with(BlockStatus::Fork);
}

#[test]
fn fail_if_duplicate() {
    BlockValidationTest::for_epoch0_account()
        .block_to_validate(|chain| chain.new_legacy_open_block().build())
        .block_already_exists()
        .assert_validation_fails_with(BlockStatus::Old);
}

#[test]
fn fail_with_gap_source_if_source_not_found() {
    BlockValidationTest::for_unopened_account()
        .block_to_validate(|chain| chain.new_legacy_open_block().build())
        .source_block_is_missing()
        .assert_validation_fails_with(BlockStatus::GapSource);
}

#[test]
fn fail_if_signature_is_bad() {
    BlockValidationTest::for_unopened_account()
        .with_pending_receive(Amount::raw(10), Epoch::Epoch0)
        .block_to_validate(|chain| {
            chain
                .new_legacy_open_block()
                .sign(&PrivateKey::new())
                .build()
        })
        .assert_validation_fails_with(BlockStatus::BadSignature);
}
