use crate::{block_insertion::validation::tests::BlockValidationTest, BlockStatus};
use rsnano_core::{
    AccountInfo, Amount, BlockDetails, BlockHash, BlockSideband, Epoch, PendingKey, PrivateKey,
};

#[test]
fn valid_open_block() {
    let test = BlockValidationTest::for_unopened_account()
        .with_pending_receive(Amount::raw(10), Epoch::Epoch2)
        .block_to_validate(|chain| chain.new_open_block().balance(10).build());
    let result = test.assert_is_valid();
    let open_block = test.block();

    assert_eq!(
        result.set_sideband,
        BlockSideband {
            height: 1,
            timestamp: test.now.as_u64(),
            successor: BlockHash::zero(),
            account: open_block.account_field().unwrap(),
            balance: open_block.balance_field().unwrap(),
            details: BlockDetails::new(Epoch::Epoch2, false, true, false),
            source_epoch: Epoch::Epoch2
        }
    );
    assert_eq!(
        result.delete_pending,
        Some(PendingKey::for_receive_block(open_block))
    );

    assert_eq!(
        result.set_account_info,
        AccountInfo {
            head: open_block.hash(),
            representative: open_block.representative_field().unwrap(),
            open_block: open_block.hash(),
            balance: open_block.balance_field().unwrap(),
            modified: test.now.as_u64(),
            block_count: 1,
            epoch: Epoch::Epoch2
        }
    )
}

#[test]
fn fails_with_fork_if_account_already_opened() {
    BlockValidationTest::for_epoch2_account()
        .with_pending_receive(Amount::raw(10), Epoch::Epoch2)
        .block_to_validate(|chain| chain.new_open_block().balance(10).build())
        .assert_validation_fails_with(BlockStatus::Fork);
}

#[test]
fn fails_with_gap_previous_if_open_block_has_previous_block() {
    BlockValidationTest::for_unopened_account()
        .with_pending_receive(Amount::raw(10), Epoch::Epoch2)
        .block_to_validate(|chain| chain.new_open_block().balance(10).previous(99999).build())
        .assert_validation_fails_with(BlockStatus::GapPrevious);
}

#[test]
fn fails_with_gap_source_if_link_missing() {
    BlockValidationTest::for_unopened_account()
        .with_pending_receive(Amount::raw(10), Epoch::Epoch2)
        .block_to_validate(|chain| chain.new_open_block().balance(10).link(0).build())
        .assert_validation_fails_with(BlockStatus::GapSource);
}

#[test]
fn fails_with_bad_signature_if_signature_is_invalid() {
    BlockValidationTest::for_unopened_account()
        .with_pending_receive(Amount::raw(10), Epoch::Epoch1)
        .block_to_validate(|chain| {
            chain
                .new_open_block()
                .balance(10)
                .link(0)
                .key(&PrivateKey::new())
                .build()
        })
        .assert_validation_fails_with(BlockStatus::BadSignature);
}
