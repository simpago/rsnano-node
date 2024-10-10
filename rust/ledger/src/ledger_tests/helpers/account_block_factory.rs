use crate::{ledger_constants::LEDGER_CONSTANTS_STUB, Ledger};
use rsnano_core::{
    Account, AccountInfo, Amount, BlockHash, Epoch, KeyPair, Link, PublicKey, DEV_GENESIS_KEY,
};

use rsnano_core::{
    BlockBuilder, LegacyChangeBlockBuilder, LegacyOpenBlockBuilder, LegacyReceiveBlockBuilder,
    LegacySendBlockBuilder, StateBlockBuilder,
};
use rsnano_store_lmdb::Transaction;

/// Test helper that creates blocks for a single account
pub struct AccountBlockFactory<'a> {
    pub key: KeyPair,
    ledger: &'a Ledger,
}

impl<'a> AccountBlockFactory<'a> {
    pub(crate) fn new(ledger: &'a Ledger) -> Self {
        Self {
            key: KeyPair::new(),
            ledger,
        }
    }

    pub(crate) fn genesis(ledger: &'a Ledger) -> Self {
        Self {
            key: DEV_GENESIS_KEY.clone(),
            ledger,
        }
    }

    pub(crate) fn public_key(&self) -> PublicKey {
        self.key.public_key()
    }

    pub fn account(&self) -> Account {
        self.key.public_key().into()
    }

    pub(crate) fn info(&self, txn: &dyn Transaction) -> Option<AccountInfo> {
        self.ledger.account_info(txn, &self.account())
    }

    pub(crate) fn legacy_open(&self, source: BlockHash) -> LegacyOpenBlockBuilder {
        BlockBuilder::legacy_open()
            .source(source)
            .representative(self.public_key())
            .account(self.account())
            .sign(&self.key)
    }

    pub(crate) fn epoch_v1(&self, txn: &dyn Transaction) -> StateBlockBuilder {
        let info = self.info(txn).unwrap();
        BlockBuilder::state()
            .account(self.account())
            .previous(info.head)
            .representative(info.representative)
            .balance(info.balance)
            .link(*LEDGER_CONSTANTS_STUB.epochs.link(Epoch::Epoch1).unwrap())
            .sign(&DEV_GENESIS_KEY)
    }

    pub(crate) fn epoch_v1_open(&self) -> StateBlockBuilder {
        BlockBuilder::state()
            .account(self.account())
            .previous(0)
            .representative(0)
            .balance(0)
            .link(self.ledger.epoch_link(Epoch::Epoch1).unwrap())
            .sign(&DEV_GENESIS_KEY)
    }

    pub(crate) fn legacy_change(&self, txn: &dyn Transaction) -> LegacyChangeBlockBuilder {
        let info = self.info(txn).unwrap();
        BlockBuilder::legacy_change()
            .previous(info.head)
            .representative(PublicKey::from(1))
            .sign(&self.key)
    }

    pub(crate) fn legacy_send(&self, txn: &dyn Transaction) -> LegacySendBlockBuilder {
        let info = self.info(txn).unwrap();
        BlockBuilder::legacy_send()
            .previous(info.head)
            .destination(Account::from(1))
            .previous_balance(info.balance)
            .amount(Amount::raw(1))
            .sign(self.key.clone())
    }

    pub(crate) fn legacy_receive(
        &self,
        txn: &dyn Transaction,
        send_hash: BlockHash,
    ) -> LegacyReceiveBlockBuilder {
        let receiver_info = self.info(txn).unwrap();
        BlockBuilder::legacy_receive()
            .previous(receiver_info.head)
            .source(send_hash)
            .sign(&self.key)
    }

    pub fn send(&self, txn: &dyn Transaction) -> StateBlockBuilder {
        let info = self.info(txn).unwrap();
        BlockBuilder::state()
            .account(self.account())
            .previous(info.head)
            .previous_balance(info.balance)
            .representative(info.representative)
            .amount_sent(Amount::raw(50))
            .link(Account::from(1))
            .sign(&self.key)
    }

    pub(crate) fn receive(&self, txn: &dyn Transaction, send_hash: BlockHash) -> StateBlockBuilder {
        let receiver_info = self.info(txn).unwrap();
        let amount_sent = self.ledger.any().block_amount(txn, &send_hash).unwrap();
        BlockBuilder::state()
            .account(self.account())
            .previous(receiver_info.head)
            .representative(receiver_info.representative)
            .balance(receiver_info.balance + amount_sent)
            .link(send_hash)
            .sign(&self.key)
    }

    pub(crate) fn change(&self, txn: &dyn Transaction) -> StateBlockBuilder {
        let info = self.info(txn).unwrap();
        BlockBuilder::state()
            .account(self.account())
            .previous(info.head)
            .representative(Account::from(1))
            .balance(info.balance)
            .link(Link::zero())
            .sign(&self.key)
    }

    pub(crate) fn open(&self, txn: &dyn Transaction, send_hash: BlockHash) -> StateBlockBuilder {
        let amount_sent = self.ledger.any().block_amount(txn, &send_hash).unwrap();
        BlockBuilder::state()
            .account(self.account())
            .previous(0)
            .representative(self.account())
            .balance(amount_sent)
            .link(send_hash)
            .sign(&self.key)
    }
}
