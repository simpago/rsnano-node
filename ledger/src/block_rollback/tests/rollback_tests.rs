use crate::block_rollback::tests::RollbackTest;
use rsnano_core::{
    Account, AccountInfo, Amount, BlockSubType, Epoch, PendingInfo, PendingKey, SavedAccountChain,
};

#[test]
fn rollback_epoch1() {
    let mut chain = SavedAccountChain::new_opened_chain();
    chain.add_epoch_v1();

    let instructions = RollbackTest::for_chain(&chain).assert_rollback_succeeds();

    assert_eq!(instructions.set_account_info.epoch, Epoch::Epoch0);
}

#[test]
fn rollback_receive_block_which_performed_epoch1_upgrade_undoes_epoch_upgrade() {
    let mut chain = SavedAccountChain::new_opened_chain();
    let receive_block = chain.new_receive_block().amount_received(1).build();
    let receive_block = chain.add_block(receive_block, Epoch::Epoch1).clone();

    let instructions = RollbackTest::for_chain(&chain)
        .with_linked_account(456)
        .assert_rollback_succeeds();

    assert_eq!(instructions.set_account_info.epoch, Epoch::Epoch0);
    assert_eq!(
        instructions.add_pending,
        Some((
            PendingKey::new(chain.account(), receive_block.link_field().unwrap().into()),
            PendingInfo {
                source: Account::from(456),
                amount: Amount::raw(1),
                epoch: Epoch::Epoch1
            }
        ))
    );
}

#[test]
fn rollback_epoch_v2() {
    let mut chain = SavedAccountChain::new_opened_chain();
    chain.add_epoch_v1();
    chain.add_epoch_v2();

    let instructions = RollbackTest::for_chain(&chain).assert_rollback_succeeds();

    assert_eq!(instructions.set_account_info.epoch, Epoch::Epoch1);
}

#[test]
fn rollback_legacy_change() {
    let mut chain = SavedAccountChain::new_opened_chain();
    let previous_account_info = chain.account_info();
    chain.add_legacy_change(123);

    let instructions = RollbackTest::for_chain(&chain).assert_rollback_succeeds();

    assert_eq!(
        instructions.set_account_info,
        AccountInfo {
            modified: RollbackTest::SECONDS_SINCE_EPOCH,
            ..previous_account_info
        }
    );
    assert_eq!(
        instructions.clear_successor,
        Some(previous_account_info.head)
    );
}

#[test]
fn rollback_legacy_open() {
    let chain = SavedAccountChain::new_opened_chain();

    let linked_account = Account::from(42);
    let instructions = RollbackTest::for_chain(&chain)
        .with_linked_account(linked_account)
        .assert_rollback_succeeds();

    assert_eq!(instructions.block_sub_type, BlockSubType::Open);
    assert_eq!(instructions.new_balance, Amount::zero());
    assert_eq!(instructions.new_representative, None);
    assert_eq!(instructions.old_account_info, chain.account_info());
    assert_eq!(instructions.set_account_info, Default::default());
    assert_eq!(instructions.clear_successor, None);
    assert_eq!(instructions.remove_pending, None);
    assert_eq!(
        instructions.add_pending,
        Some((
            PendingKey::new(
                chain.account(),
                chain.latest_block().source_or_link().into()
            ),
            PendingInfo {
                source: linked_account,
                amount: chain.account_info().balance,
                epoch: Epoch::Epoch0
            }
        ))
    );
}
