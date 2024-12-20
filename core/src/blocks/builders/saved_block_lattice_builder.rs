use crate::{
    blocks::{
        open_block::OpenBlockArgs, receive_block::ReceiveBlockArgs, send_block::SendBlockArgs,
        state_block::EpochBlockArgs,
    },
    dev_epoch1_signer, epoch_v1_link,
    utils::UnixTimestamp,
    work::{WorkPool, WorkPoolImpl},
    Account, Amount, Block, BlockDetails, BlockHash, BlockSideband, ChangeBlockArgs, Epoch, Link,
    PendingInfo, PendingKey, PrivateKey, PublicKey, Root, SavedBlock, StateBlockArgs,
    DEV_GENESIS_BLOCK, DEV_GENESIS_KEY,
};
use std::collections::HashMap;

pub struct SavedBlockLatticeBuilder {
    accounts: HashMap<Account, Frontier>,
    work_pool: WorkPoolImpl,
    pending_receives: HashMap<PendingKey, PendingInfo>,
    now: UnixTimestamp,
}

#[derive(Clone, Default)]
struct Frontier {
    hash: BlockHash,
    representative: PublicKey,
    balance: Amount,
    height: u64,
}

impl SavedBlockLatticeBuilder {
    pub fn new() -> Self {
        let mut accounts = HashMap::new();
        accounts.insert(
            DEV_GENESIS_KEY.account(),
            Frontier {
                hash: DEV_GENESIS_BLOCK.hash(),
                representative: DEV_GENESIS_KEY.public_key(),
                balance: Amount::MAX,
                height: 1,
            },
        );
        let work_pool = WorkPoolImpl::new_dev();
        Self {
            accounts,
            work_pool,
            pending_receives: Default::default(),
            now: UnixTimestamp::new(42),
        }
    }

    pub fn set_now(&mut self, now: UnixTimestamp) {
        self.now = now;
    }

    pub fn advance_time(&mut self) {
        self.now = self.now.add(1);
    }

    pub fn genesis(&mut self) -> SavedAccountChainBuilder {
        self.account(&DEV_GENESIS_KEY)
    }

    pub fn account<'a>(&'a mut self, key: &'a PrivateKey) -> SavedAccountChainBuilder<'a> {
        SavedAccountChainBuilder { lattice: self, key }
    }

    pub fn epoch_open(&mut self, account: impl Into<Account>) -> SavedBlock {
        let account = account.into();
        assert!(!self.accounts.contains_key(&account));
        assert!(self
            .pending_receives
            .keys()
            .any(|k| k.receiving_account == account));

        let receive: Block = EpochBlockArgs {
            epoch_signer: dev_epoch1_signer(),
            account,
            previous: BlockHash::zero(),
            representative: PublicKey::zero(),
            balance: Amount::zero(),
            link: epoch_v1_link(),
            work: self.work_pool.generate_dev2(account.into()).unwrap(),
        }
        .into();

        let new_frontier = Frontier {
            hash: receive.hash(),
            representative: PublicKey::zero(),
            balance: Amount::zero(),
            height: 1,
        };

        let sideband = self.sideband_for(account, &Frontier::default(), &new_frontier);

        self.accounts.insert(account, new_frontier);

        SavedBlock::new(receive, sideband)
    }

    fn sideband_for(
        &self,
        account: Account,
        old_frontier: &Frontier,
        new_frontier: &Frontier,
    ) -> BlockSideband {
        let is_send = new_frontier.balance < old_frontier.balance;
        let is_receive = new_frontier.balance > old_frontier.balance;

        BlockSideband {
            height: new_frontier.height,
            timestamp: self.now,
            successor: BlockHash::zero(),
            account,
            balance: new_frontier.balance,
            details: BlockDetails::new(Epoch::Epoch0, is_send, is_receive, false), //TODO epoch
            source_epoch: Epoch::Epoch0,                                           //TODO
        }
    }

    fn pop_pending_receive(
        &mut self,
        receiving_account: impl Into<Account>,
        send_hash: BlockHash,
    ) -> PendingInfo {
        self.pending_receives
            .remove(&PendingKey::new(receiving_account.into(), send_hash))
            .expect("no pending receive found")
    }
}

impl Clone for SavedBlockLatticeBuilder {
    fn clone(&self) -> Self {
        Self {
            accounts: self.accounts.clone(),
            work_pool: WorkPoolImpl::new_dev(),
            pending_receives: self.pending_receives.clone(),
            now: self.now,
        }
    }
}

pub struct SavedAccountChainBuilder<'a> {
    lattice: &'a mut SavedBlockLatticeBuilder,
    key: &'a PrivateKey,
}

impl<'a> SavedAccountChainBuilder<'a> {
    pub fn send_max(&mut self, destination: impl Into<Account>) -> SavedBlock {
        self.send_all_except(destination, 0)
    }

    pub fn send_all_except(
        &mut self,
        destination: impl Into<Account>,
        keep: impl Into<Amount>,
    ) -> SavedBlock {
        let frontier = self.get_frontier();
        self.send(destination, frontier.balance - keep.into())
    }

    pub fn send(
        &mut self,
        destination: impl Into<Account>,
        amount: impl Into<Amount>,
    ) -> SavedBlock {
        let destination = destination.into();
        let old_frontier = self.get_frontier();
        let amount = amount.into();
        let new_balance = old_frontier.balance - amount;

        let send: Block = StateBlockArgs {
            key: self.key,
            previous: old_frontier.hash,
            representative: old_frontier.representative,
            balance: new_balance,
            link: destination.into(),
            work: self
                .lattice
                .work_pool
                .generate_dev2(old_frontier.hash.into())
                .unwrap(),
        }
        .into();

        let new_frontier = Frontier {
            hash: send.hash(),
            representative: old_frontier.representative,
            balance: new_balance,
            height: old_frontier.height + 1,
        };

        let sideband = self
            .lattice
            .sideband_for(self.key.account(), &old_frontier, &new_frontier);

        self.set_new_frontier(new_frontier);
        self.lattice.pending_receives.insert(
            PendingKey::new(destination, send.hash()),
            PendingInfo {
                source: self.key.account(),
                amount,
                epoch: Epoch::Epoch0,
            },
        );

        SavedBlock::new(send, sideband)
    }

    pub fn legacy_send(
        &mut self,
        destination: impl Into<Account>,
        amount: impl Into<Amount>,
    ) -> SavedBlock {
        let destination = destination.into();
        let old_frontier = self.get_frontier();
        let amount = amount.into();
        let new_balance = old_frontier.balance - amount;

        let work = self
            .lattice
            .work_pool
            .generate_dev2(old_frontier.hash.into())
            .unwrap();

        let send: Block = SendBlockArgs {
            key: self.key,
            previous: old_frontier.hash,
            destination,
            balance: new_balance,
            work,
        }
        .into();

        let new_frontier = Frontier {
            hash: send.hash(),
            balance: new_balance,
            height: old_frontier.height,
            ..old_frontier
        };

        let sideband = self
            .lattice
            .sideband_for(self.key.account(), &old_frontier, &new_frontier);

        self.set_new_frontier(new_frontier);

        self.lattice.pending_receives.insert(
            PendingKey::new(destination, send.hash()),
            PendingInfo {
                source: self.key.account(),
                amount,
                epoch: Epoch::Epoch0,
            },
        );

        SavedBlock::new(send, sideband)
    }

    pub fn legacy_open(&mut self, corresponding_send: &Block) -> SavedBlock {
        self.legacy_open_with_rep(corresponding_send, self.key.public_key())
    }

    pub fn legacy_open_with_rep(
        &mut self,
        corresponding_send: &Block,
        new_representative: impl Into<PublicKey>,
    ) -> SavedBlock {
        assert!(!self.lattice.accounts.contains_key(&self.key.account()));
        assert_eq!(corresponding_send.destination_or_link(), self.key.account());

        let amount = self
            .lattice
            .pop_pending_receive(self.key, corresponding_send.hash())
            .amount;

        let root: Root = self.key.account().into();

        let work = self.lattice.work_pool.generate_dev2(root).unwrap();
        let receive: Block = OpenBlockArgs {
            key: &self.key,
            source: corresponding_send.hash(),
            representative: new_representative.into(),
            work,
        }
        .into();

        let old_frontier = Frontier::default();

        let new_frontier = Frontier {
            hash: receive.hash(),
            representative: self.key.public_key(),
            balance: amount,
            height: 1,
        };

        let sideband = self
            .lattice
            .sideband_for(self.key.account(), &old_frontier, &new_frontier);

        self.set_new_frontier(new_frontier);

        SavedBlock::new(receive, sideband)
    }

    pub fn legacy_receive(&mut self, corresponding_send: &Block) -> SavedBlock {
        let frontier = self.get_frontier();
        self.legacy_receive_with_rep(corresponding_send, frontier.representative)
    }

    pub fn legacy_receive_with_rep(
        &mut self,
        corresponding_send: &Block,
        new_representative: impl Into<PublicKey>,
    ) -> SavedBlock {
        assert_eq!(corresponding_send.destination_or_link(), self.key.account());
        let amount = self
            .lattice
            .pop_pending_receive(self.key, corresponding_send.hash())
            .amount;

        let old_frontier = self.get_frontier();
        let root: Root = old_frontier.hash.into();
        let new_balance = old_frontier.balance + amount;
        let work = self.lattice.work_pool.generate_dev2(root).unwrap();

        let receive: Block = ReceiveBlockArgs {
            key: self.key,
            previous: old_frontier.hash,
            source: corresponding_send.hash(),
            work,
        }
        .into();

        let new_frontier = Frontier {
            hash: receive.hash(),
            representative: new_representative.into(),
            balance: new_balance,
            height: old_frontier.height + 1,
        };

        let sideband = self
            .lattice
            .sideband_for(self.key.account(), &old_frontier, &new_frontier);

        self.set_new_frontier(new_frontier);

        SavedBlock::new(receive, sideband)
    }

    pub fn receive(&mut self, corresponding_send: &Block) -> SavedBlock {
        let frontier = self.get_frontier_or_empty();
        self.receive_and_change(corresponding_send, frontier.representative)
    }

    pub fn receive_and_change(
        &mut self,
        corresponding_send: &Block,
        new_representative: impl Into<PublicKey>,
    ) -> SavedBlock {
        assert_eq!(corresponding_send.destination_or_link(), self.key.account());
        let amount = self
            .lattice
            .pop_pending_receive(self.key, corresponding_send.hash())
            .amount;

        let old_frontier = self.get_frontier_or_empty();

        let root: Root = if old_frontier.hash.is_zero() {
            self.key.account().into()
        } else {
            old_frontier.hash.into()
        };

        let new_balance = old_frontier.balance + amount;

        let receive: Block = StateBlockArgs {
            key: self.key,
            previous: old_frontier.hash,
            representative: new_representative.into(),
            balance: new_balance,
            link: corresponding_send.hash().into(),
            work: self.lattice.work_pool.generate_dev2(root).unwrap(),
        }
        .into();

        let new_frontier = Frontier {
            hash: receive.hash(),
            representative: old_frontier.representative,
            balance: new_balance,
            height: old_frontier.height + 1,
        };

        let sideband = self
            .lattice
            .sideband_for(self.key.account(), &old_frontier, &new_frontier);

        self.set_new_frontier(new_frontier);

        SavedBlock::new(receive, sideband)
    }

    pub fn legacy_change(&mut self, new_representative: impl Into<PublicKey>) -> SavedBlock {
        let old_frontier = self.get_frontier();
        let new_representative = new_representative.into();
        let work = self
            .lattice
            .work_pool
            .generate_dev2(old_frontier.hash.into())
            .unwrap();

        let change: Block = ChangeBlockArgs {
            key: self.key,
            previous: old_frontier.hash,
            representative: new_representative,
            work,
        }
        .into();

        let new_frontier = Frontier {
            hash: change.hash(),
            representative: new_representative,
            height: old_frontier.height + 1,
            ..old_frontier
        };

        let sideband = self
            .lattice
            .sideband_for(self.key.account(), &old_frontier, &new_frontier);

        self.set_new_frontier(new_frontier);

        SavedBlock::new(change, sideband)
    }

    pub fn change(&mut self, new_representative: impl Into<PublicKey>) -> SavedBlock {
        let old_frontier = self.get_frontier();
        let new_representative = new_representative.into();
        let change: Block = StateBlockArgs {
            key: self.key,
            previous: old_frontier.hash,
            representative: new_representative,
            balance: old_frontier.balance,
            link: Link::zero(),
            work: self
                .lattice
                .work_pool
                .generate_dev2(old_frontier.hash.into())
                .unwrap(),
        }
        .into();

        let new_frontier = Frontier {
            hash: change.hash(),
            representative: new_representative,
            balance: old_frontier.balance,
            height: old_frontier.height + 1,
        };
        let sideband = self
            .lattice
            .sideband_for(self.key.account(), &old_frontier, &new_frontier);
        self.set_new_frontier(new_frontier);

        SavedBlock::new(change, sideband)
    }

    pub fn epoch1(&mut self) -> SavedBlock {
        let old_frontier = self.get_frontier();
        let epoch: Block = EpochBlockArgs {
            epoch_signer: dev_epoch1_signer(),
            account: self.key.account(),
            previous: old_frontier.hash,
            representative: old_frontier.representative,
            balance: old_frontier.balance,
            link: epoch_v1_link(),
            work: self
                .lattice
                .work_pool
                .generate_dev2(old_frontier.hash.into())
                .unwrap(),
        }
        .into();

        let new_frontier = Frontier {
            hash: epoch.hash(),
            height: old_frontier.height + 1,
            ..old_frontier
        };

        let sideband = self
            .lattice
            .sideband_for(self.key.account(), &old_frontier, &new_frontier);
        self.set_new_frontier(new_frontier);

        SavedBlock::new(epoch, sideband)
    }

    fn set_new_frontier(&mut self, new_frontier: Frontier) {
        self.lattice
            .accounts
            .insert(self.key.account(), new_frontier);
    }

    fn get_frontier(&self) -> Frontier {
        self.lattice
            .accounts
            .get(&self.key.account())
            .expect("Cannot send/change from unopenend account!")
            .clone()
    }

    fn get_frontier_or_empty(&self) -> Frontier {
        self.lattice
            .accounts
            .get(&self.key.account())
            .cloned()
            .unwrap_or_else(|| Frontier {
                hash: BlockHash::zero(),
                representative: self.key.public_key(),
                balance: Amount::zero(),
                height: 0,
            })
    }
}
