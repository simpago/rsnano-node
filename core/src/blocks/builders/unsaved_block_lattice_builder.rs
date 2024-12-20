use super::saved_block_lattice_builder::{SavedAccountChainBuilder, SavedBlockLatticeBuilder};
use crate::{Account, Amount, Block, PrivateKey, PublicKey, DEV_GENESIS_KEY};

#[derive(Clone)]
pub struct UnsavedBlockLatticeBuilder {
    inner: SavedBlockLatticeBuilder,
}

impl UnsavedBlockLatticeBuilder {
    pub fn new() -> Self {
        Self {
            inner: SavedBlockLatticeBuilder::new(),
        }
    }

    pub fn genesis(&mut self) -> UnsavedAccountChainBuilder {
        self.account(&DEV_GENESIS_KEY)
    }

    pub fn account<'a>(&'a mut self, key: &'a PrivateKey) -> UnsavedAccountChainBuilder<'a> {
        UnsavedAccountChainBuilder {
            inner: self.inner.account(key),
        }
    }

    pub fn epoch_open(&mut self, account: impl Into<Account>) -> Block {
        self.inner.epoch_open(account).block
    }
}

pub struct UnsavedAccountChainBuilder<'a> {
    inner: SavedAccountChainBuilder<'a>,
}

impl<'a> UnsavedAccountChainBuilder<'a> {
    pub fn send_max(&mut self, destination: impl Into<Account>) -> Block {
        self.inner.send_max(destination).block
    }

    pub fn send_all_except(
        &mut self,
        destination: impl Into<Account>,
        keep: impl Into<Amount>,
    ) -> Block {
        self.inner.send_all_except(destination, keep).block
    }

    pub fn send(&mut self, destination: impl Into<Account>, amount: impl Into<Amount>) -> Block {
        self.inner.send(destination, amount).block
    }

    pub fn legacy_send(
        &mut self,
        destination: impl Into<Account>,
        amount: impl Into<Amount>,
    ) -> Block {
        self.inner.legacy_send(destination, amount).block
    }

    pub fn legacy_open(&mut self, corresponding_send: &Block) -> Block {
        self.inner.legacy_open(corresponding_send).block
    }

    pub fn legacy_open_with_rep(
        &mut self,
        corresponding_send: &Block,
        new_representative: impl Into<PublicKey>,
    ) -> Block {
        self.inner
            .legacy_open_with_rep(corresponding_send, new_representative)
            .block
    }

    pub fn legacy_receive(&mut self, corresponding_send: &Block) -> Block {
        self.inner.legacy_receive(corresponding_send).block
    }

    pub fn legacy_receive_with_rep(
        &mut self,
        corresponding_send: &Block,
        new_representative: impl Into<PublicKey>,
    ) -> Block {
        self.inner
            .legacy_open_with_rep(corresponding_send, new_representative)
            .block
    }

    pub fn receive(&mut self, corresponding_send: &Block) -> Block {
        self.inner.receive(corresponding_send).block
    }

    pub fn receive_and_change(
        &mut self,
        corresponding_send: &Block,
        new_representative: impl Into<PublicKey>,
    ) -> Block {
        self.inner
            .receive_and_change(corresponding_send, new_representative)
            .block
    }

    pub fn legacy_change(&mut self, new_representative: impl Into<PublicKey>) -> Block {
        self.inner.legacy_change(new_representative).block
    }

    pub fn change(&mut self, new_representative: impl Into<PublicKey>) -> Block {
        self.inner.change(new_representative).block
    }

    pub fn epoch1(&mut self) -> Block {
        self.inner.epoch1().block
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{work::WorkThresholds, BlockDetails, BlockHash, StateBlockArgs, DEV_GENESIS_BLOCK};

    #[test]
    fn state_send() {
        let mut lattice = UnsavedBlockLatticeBuilder::new();
        let key1 = PrivateKey::from(42);

        let send = lattice.genesis().send(&key1, 1);

        let expected: Block = StateBlockArgs {
            key: &DEV_GENESIS_KEY,
            previous: DEV_GENESIS_BLOCK.hash(),
            representative: DEV_GENESIS_KEY.public_key(),
            balance: Amount::MAX - Amount::raw(1),
            link: key1.account().into(),
            work: send.work(),
        }
        .into();
        assert_eq!(send, expected);
        assert!(WorkThresholds::publish_dev().is_valid_pow(
            &send,
            &BlockDetails::new(crate::Epoch::Epoch2, true, false, false)
        ))
    }

    #[test]
    fn send_twice() {
        let mut lattice = UnsavedBlockLatticeBuilder::new();
        let key1 = PrivateKey::from(42);

        let send1 = lattice.genesis().send(&key1, 1);
        let send2 = lattice.genesis().send(&key1, 2);

        let expected: Block = StateBlockArgs {
            key: &DEV_GENESIS_KEY,
            previous: send1.hash(),
            representative: DEV_GENESIS_KEY.public_key(),
            balance: Amount::MAX - Amount::raw(3),
            link: key1.account().into(),
            work: send2.work(),
        }
        .into();
        assert_eq!(send2, expected);
    }

    #[test]
    fn state_open() {
        let mut lattice = UnsavedBlockLatticeBuilder::new();
        let key1 = PrivateKey::from(42);
        let send = lattice.genesis().send(&key1, 1);

        let open = lattice.account(&key1).receive(&send);

        let expected: Block = StateBlockArgs {
            key: &key1,
            previous: BlockHash::zero(),
            representative: key1.public_key(),
            balance: Amount::raw(1),
            link: send.hash().into(),
            work: open.work(),
        }
        .into();
        assert_eq!(open, expected);
        assert!(WorkThresholds::publish_dev().is_valid_pow(
            &send,
            &BlockDetails::new(crate::Epoch::Epoch2, false, true, false)
        ))
    }

    #[test]
    fn state_receive() {
        let mut lattice = UnsavedBlockLatticeBuilder::new();
        let key1 = PrivateKey::from(42);
        let send1 = lattice.genesis().send(&key1, 1);
        let send2 = lattice.genesis().send(&key1, 2);
        let open = lattice.account(&key1).receive(&send1);

        let receive = lattice.account(&key1).receive(&send2);

        let expected: Block = StateBlockArgs {
            key: &key1,
            previous: open.hash(),
            representative: key1.public_key(),
            balance: Amount::raw(3),
            link: send2.hash().into(),
            work: receive.work(),
        }
        .into();
        assert_eq!(receive, expected);
    }
}
