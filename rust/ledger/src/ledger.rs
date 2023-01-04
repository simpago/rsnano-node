use crate::{
    block_insertion::{BlockInserter, BlockValidatorFactory},
    BlockRollbackPerformer, GenerateCache, LedgerCache, LedgerConstants, RepWeights,
    RepresentativeBlockFinder,
};
use rand::{thread_rng, Rng};
use rsnano_core::{
    utils::seconds_since_epoch, Account, AccountInfo, Amount, Block, BlockEnum, BlockHash,
    BlockSubType, BlockType, ConfirmationHeightInfo, Epoch, Link, PendingInfo, PendingKey,
    QualifiedRoot, Root,
};

use std::{
    collections::{BTreeMap, HashMap},
    ops::Deref,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc, Mutex,
    },
};

use super::DependentBlocksFinder;
use rsnano_store_traits::{ReadTransaction, Store, Transaction, WriteTransaction};

#[derive(Debug, PartialEq, Eq)]
pub struct UncementedInfo {
    pub cemented_frontier: BlockHash,
    pub frontier: BlockHash,
    pub account: Account,
}

#[derive(PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum ProcessResult {
    Progress,      // Hasn't been seen before, signed correctly
    BadSignature,  // Signature was bad, forged or transmission error
    Old,           // Already seen and was valid
    NegativeSpend, // Malicious attempt to spend a negative amount
    Fork,          // Malicious fork based on previous
    /// Source block doesn't exist, has already been received, or requires an account upgrade (epoch blocks)
    Unreceivable,
    GapPrevious,         // Block marked as previous is unknown
    GapSource,           // Block marked as source is unknown
    GapEpochOpenPending, // Block marked as pending blocks required for epoch open block are unknown
    OpenedBurnAccount,   // Block attempts to open the burn account
    /// Balance and amount delta don't match
    BalanceMismatch,
    RepresentativeMismatch, // Representative is changed when it is not allowed
    BlockPosition,          // This block cannot follow the previous block
    InsufficientWork, // Insufficient work for this block, even though it passed the minimal validation
}

pub trait LedgerObserver: Send + Sync {
    fn blocks_cemented(&self, _cemented_count: u64) {}
    fn block_rolled_back(&self, _block_type: BlockSubType) {}
    fn block_rolled_back2(&self, _block: &BlockEnum, _is_epoch: bool) {}
    fn block_added(&self, _block: &BlockEnum, _is_epoch: bool) {}
}

pub struct NullLedgerObserver {}

impl NullLedgerObserver {
    pub fn new() -> Self {
        Self {}
    }
}

impl LedgerObserver for NullLedgerObserver {}

pub struct Ledger {
    pub store: Arc<dyn Store>,
    pub cache: Arc<LedgerCache>,
    pub constants: LedgerConstants,
    pub observer: Arc<dyn LedgerObserver>,
    pruning: AtomicBool,
    bootstrap_weight_max_blocks: AtomicU64,
    pub check_bootstrap_weights: AtomicBool,
    pub bootstrap_weights: Mutex<HashMap<Account, Amount>>,
}

impl Ledger {
    pub fn new(store: Arc<dyn Store>, constants: LedgerConstants) -> anyhow::Result<Self> {
        Self::with_cache(store, constants, &GenerateCache::new())
    }

    pub fn with_cache(
        store: Arc<dyn Store>,
        constants: LedgerConstants,
        generate_cache: &GenerateCache,
    ) -> anyhow::Result<Self> {
        let mut ledger = Self {
            store,
            cache: Arc::new(LedgerCache::new()),
            constants,
            observer: Arc::new(NullLedgerObserver::new()),
            pruning: AtomicBool::new(false),
            bootstrap_weight_max_blocks: AtomicU64::new(1),
            check_bootstrap_weights: AtomicBool::new(true),
            bootstrap_weights: Mutex::new(HashMap::new()),
        };

        ledger.initialize(generate_cache)?;

        Ok(ledger)
    }

    pub fn set_observer(&mut self, observer: Arc<dyn LedgerObserver>) {
        self.observer = observer;
    }

    pub fn read_txn(&self) -> Box<dyn ReadTransaction> {
        self.store.tx_begin_read().unwrap()
    }

    pub fn rw_txn(&self) -> Box<dyn WriteTransaction> {
        self.store.tx_begin_write().unwrap()
    }

    fn initialize(&mut self, generate_cache: &GenerateCache) -> anyhow::Result<()> {
        if self.store.account().begin(self.read_txn().txn()).is_end() {
            self.add_genesis_block(self.rw_txn().as_mut());
        }

        if generate_cache.reps || generate_cache.account_count || generate_cache.block_count {
            self.store.account().for_each_par(&|_txn, mut i, n| {
                let mut block_count = 0;
                let mut account_count = 0;
                let rep_weights = RepWeights::new();
                while !i.eq(n.as_ref()) {
                    let info = i.current().unwrap().1;
                    block_count += info.block_count;
                    account_count += 1;
                    rep_weights.representation_add(info.representative, info.balance);
                    i.next();
                }
                self.cache
                    .block_count
                    .fetch_add(block_count, Ordering::SeqCst);
                self.cache
                    .account_count
                    .fetch_add(account_count, Ordering::SeqCst);
                self.cache.rep_weights.copy_from(&rep_weights);
            });
        }

        if generate_cache.cemented_count {
            self.store
                .confirmation_height()
                .for_each_par(&|_txn, mut i, n| {
                    let mut cemented_count = 0;
                    while !i.eq(n.as_ref()) {
                        cemented_count += i.current().unwrap().1.height;
                        i.next();
                    }
                    self.cache
                        .cemented_count
                        .fetch_add(cemented_count, Ordering::SeqCst);
                });
        }

        let transaction = self.store.tx_begin_read()?;
        self.cache.pruned_count.fetch_add(
            self.store.pruned().count(transaction.txn()) as u64,
            Ordering::SeqCst,
        );

        // Final votes requirement for confirmation canary block
        if let Some(conf_height) = self.store.confirmation_height().get(
            transaction.txn(),
            &self.constants.final_votes_canary_account,
        ) {
            self.cache.final_votes_confirmation_canary.store(
                conf_height.height >= self.constants.final_votes_canary_height,
                Ordering::SeqCst,
            );
        }
        Ok(())
    }

    fn add_genesis_block(&self, txn: &mut dyn WriteTransaction) {
        let genesis_block_enum = self.constants.genesis.read().unwrap();
        let genesis_block = genesis_block_enum.deref();
        let genesis_hash = genesis_block.hash();
        let genesis_account = genesis_block.account();
        self.store.block().put(txn, genesis_block);
        self.store.confirmation_height().put(
            txn,
            &genesis_account,
            &ConfirmationHeightInfo::new(1, genesis_hash),
        );
        self.store.account().put(
            txn,
            &genesis_account,
            &AccountInfo {
                head: genesis_hash,
                representative: genesis_account,
                open_block: genesis_hash,
                balance: u128::MAX.into(),
                modified: seconds_since_epoch(),
                block_count: 1,
                epoch: Epoch::Epoch0,
            },
        );
        self.store
            .frontier()
            .put(txn, &genesis_hash, &genesis_account);
    }

    pub fn pruning_enabled(&self) -> bool {
        self.pruning.load(Ordering::SeqCst)
    }

    pub fn enable_pruning(&self) {
        self.pruning.store(true, Ordering::SeqCst);
    }

    pub fn bootstrap_weight_max_blocks(&self) -> u64 {
        self.bootstrap_weight_max_blocks.load(Ordering::SeqCst)
    }

    pub fn set_bootstrap_weight_max_blocks(&self, max: u64) {
        self.bootstrap_weight_max_blocks
            .store(max, Ordering::SeqCst)
    }

    pub fn block_or_pruned_exists(&self, block: &BlockHash) -> bool {
        let txn = self.store.tx_begin_read().unwrap();
        self.block_or_pruned_exists_txn(txn.txn(), block)
    }

    pub fn block_or_pruned_exists_txn(&self, txn: &dyn Transaction, hash: &BlockHash) -> bool {
        self.store.pruned().exists(txn, hash) || self.store.block().exists(txn, hash)
    }

    /// Balance for account containing the given block at the time of the block.
    /// Returns 0 if the block was not found
    pub fn balance(&self, txn: &dyn Transaction, hash: &BlockHash) -> Amount {
        if hash.is_zero() {
            Amount::zero()
        } else {
            self.store.block().balance(txn, hash)
        }
    }

    /// Balance for account containing the given block at the time of the block.
    /// Returns Err if the pruning is enabled and the block was not found.
    pub fn balance_safe(&self, txn: &dyn Transaction, hash: &BlockHash) -> anyhow::Result<Amount> {
        if self.pruning_enabled() && !hash.is_zero() && !self.store.block().exists(txn, hash) {
            bail!("block not found");
        }

        Ok(self.balance(txn, hash))
    }

    /// Balance for account by account number
    pub fn account_balance(
        &self,
        txn: &dyn Transaction,
        account: &Account,
        only_confirmed: bool,
    ) -> Amount {
        if only_confirmed {
            match self.store.confirmation_height().get(txn, account) {
                Some(info) => self.balance(txn, &info.frontier),
                None => Amount::zero(),
            }
        } else {
            match self.store.account().get(txn, account) {
                Some(info) => info.balance,
                None => Amount::zero(),
            }
        }
    }

    pub fn account_receivable(
        &self,
        txn: &dyn Transaction,
        account: &Account,
        only_confirmed: bool,
    ) -> Amount {
        let mut result = Amount::zero();
        let end = Account::from(account.number() + 1);
        let mut i = self
            .store
            .pending()
            .begin_at_key(txn, &PendingKey::new(*account, BlockHash::zero()));
        let n = self
            .store
            .pending()
            .begin_at_key(txn, &PendingKey::new(end, BlockHash::zero()));
        while !i.eq(n.as_ref()) {
            if let Some((key, info)) = i.current() {
                if only_confirmed {
                    if self.block_confirmed(txn, &key.hash) {
                        result += info.amount;
                    }
                } else {
                    result += info.amount;
                }
            };
            i.next();
        }

        result
    }

    pub fn block_confirmed(&self, txn: &dyn Transaction, hash: &BlockHash) -> bool {
        if self.store.pruned().exists(txn, hash) {
            return true;
        }

        match self.store.block().get(txn, hash) {
            Some(block) => {
                let mut account = block.account();
                let sideband = &block.sideband().unwrap();
                if account.is_zero() {
                    account = sideband.account;
                }
                let confirmed = match self.store.confirmation_height().get(txn, &account) {
                    Some(info) => info.height >= sideband.height,
                    None => false,
                };
                confirmed
            }
            None => false,
        }
    }

    pub fn block_text(&self, hash: &BlockHash) -> anyhow::Result<String> {
        let txn = self.store.tx_begin_read()?;
        match self.store.block().get(txn.txn(), hash) {
            Some(block) => block.to_json(),
            None => Ok(String::new()),
        }
    }

    pub fn is_send(&self, txn: &dyn Transaction, block: &dyn Block) -> bool {
        if block.block_type() != BlockType::State {
            return block.block_type() == BlockType::LegacySend;
        }
        let previous = block.previous();
        /*
         * if block_a does not have a sideband, then is_send()
         * requires that the previous block exists in the database.
         * This is because it must retrieve the balance of the previous block.
         */
        debug_assert!(
            block.sideband().is_some()
                || previous.is_zero()
                || self.store.block().exists(txn, &previous)
        );
        match block.sideband() {
            Some(sideband) => sideband.details.is_send,
            None => {
                if !previous.is_zero() {
                    block.balance() < self.balance(txn, &previous)
                } else {
                    false
                }
            }
        }
    }

    pub fn block_destination(&self, txn: &dyn Transaction, block: &BlockEnum) -> Account {
        match block {
            BlockEnum::LegacySend(send) => send.hashables.destination,
            BlockEnum::State(state) => {
                if self.is_send(txn, state) {
                    state.link().into()
                } else {
                    Account::zero()
                }
            }
            _ => Account::zero(),
        }
    }

    pub fn block_source(&self, txn: &dyn Transaction, block: &BlockEnum) -> BlockHash {
        /*
         * block_source() requires that the previous block of the block
         * passed in exist in the database.  This is because it will try
         * to check account balances to determine if it is a send block.
         */
        debug_assert!(
            block.previous().is_zero() || self.store.block().exists(txn, &block.previous())
        );

        // If block_a.source () is nonzero, then we have our source.
        // However, universal blocks will always return zero.
        match block {
            BlockEnum::State(state) => {
                if !self.is_send(txn, state) {
                    state.link().into()
                } else {
                    state.source()
                }
            }
            _ => block.source().unwrap_or_default(),
        }
    }

    pub fn hash_root_random(&self, txn: &dyn Transaction) -> Option<(BlockHash, BlockHash)> {
        if !self.pruning_enabled() {
            self.store
                .block()
                .random(txn)
                .map(|block| (block.hash(), block.root().into()))
        } else {
            let mut hash = BlockHash::zero();
            let count = self.cache.block_count.load(Ordering::SeqCst);
            let region = thread_rng().gen_range(0..count);
            // Pruned cache cannot guarantee that pruned blocks are already commited
            if region < self.cache.pruned_count.load(Ordering::SeqCst) {
                hash = self.store.pruned().random(txn).unwrap_or_default();
            }
            if hash.is_zero() {
                self.store
                    .block()
                    .random(txn)
                    .map(|block| (block.hash(), block.root().into()))
            } else {
                Some((hash, BlockHash::zero()))
            }
        }
    }

    /// Vote weight of an account
    pub fn weight(&self, account: &Account) -> Amount {
        if self.check_bootstrap_weights.load(Ordering::SeqCst) {
            if self.cache.block_count.load(Ordering::SeqCst) < self.bootstrap_weight_max_blocks() {
                let weights = self.bootstrap_weights.lock().unwrap();
                //println!("weights: {:?}", weights);
                if let Some(&weight) = weights.get(account) {
                    return weight;
                }
            } else {
                self.check_bootstrap_weights.store(false, Ordering::SeqCst);
            }
        }
        //println!("cache: {:?}", *self.cache.rep_weights);
        //println!("gen: {:?}", self.cache.rep_weights.representation_get(account));
        self.cache.rep_weights.representation_get(account)
    }

    /// Return account containing block hash
    pub fn account(&self, txn: &dyn Transaction, hash: &BlockHash) -> Option<Account> {
        self.store.block().account(txn, hash)
    }

    /// Return absolute amount decrease or increase for block
    pub fn amount(&self, txn: &dyn Transaction, hash: &BlockHash) -> Option<Amount> {
        self.store.block().get(txn, hash).map(|block| {
            let block_balance = self.balance(txn, hash);
            let previous_balance = self.balance(txn, &block.previous());
            if block_balance > previous_balance {
                block_balance - previous_balance
            } else {
                previous_balance - block_balance
            }
        })
    }

    /// Return absolute amount decrease or increase for block
    pub fn amount_safe(&self, txn: &dyn Transaction, hash: &BlockHash) -> Option<Amount> {
        self.store
            .block()
            .get(txn, hash)
            .map(|block| {
                let block_balance = self.balance(txn, hash);
                let previous_balance = self.balance_safe(txn, &block.previous());
                match previous_balance {
                    Ok(previous) => {
                        if block_balance > previous {
                            Some(block_balance - previous)
                        } else {
                            Some(previous - block_balance)
                        }
                    }
                    Err(_) => None,
                }
            })
            .flatten()
    }

    /// Return latest block for account
    pub fn latest(&self, txn: &dyn Transaction, account: &Account) -> Option<BlockHash> {
        self.store.account().get(txn, account).map(|info| info.head)
    }

    /// Return latest root for account, account number if there are no blocks for this account
    pub fn latest_root(&self, txn: &dyn Transaction, account: &Account) -> Root {
        match self.store.account().get(txn, account) {
            Some(info) => info.head.into(),
            None => account.into(),
        }
    }

    pub fn is_epoch_link(&self, link: &Link) -> bool {
        self.constants.epochs.is_epoch_link(link)
    }

    /// Given the block hash of a send block, find the associated receive block that receives that send.
    /// The send block hash is not checked in any way, it is assumed to be correct.
    /// Return the receive block on success and None on failure
    pub fn find_receive_block_by_send_hash(
        &self,
        txn: &dyn Transaction,
        destination: &Account,
        send_block_hash: &BlockHash,
    ) -> Option<BlockEnum> {
        // get the cemented frontier
        let info = self.store.confirmation_height().get(txn, destination)?;
        let mut possible_receive_block = self.store.block().get(txn, &info.frontier);

        // walk down the chain until the source field of a receive block matches the send block hash
        while let Some(current) = possible_receive_block {
            // if source is non-zero then it is a legacy receive or open block
            let mut source = current.source().unwrap_or_default();

            // if source is zero then it could be a state block, which needs a different kind of access
            if let BlockEnum::State(state_block) = &current {
                // we read the block from the database, so we expect it to have sideband
                if state_block.sideband().unwrap().details.is_receive {
                    source = state_block.link().into();
                }
            }

            if *send_block_hash == source {
                // we have a match
                return Some(current);
            }

            possible_receive_block = self.store.block().get(txn, &current.previous());
        }

        None
    }

    pub fn epoch_link(&self, epoch: Epoch) -> Option<Link> {
        self.constants.epochs.link(epoch).cloned()
    }

    pub fn update_account(
        &self,
        txn: &mut dyn WriteTransaction,
        account: &Account,
        old_info: &AccountInfo,
        new_info: &AccountInfo,
    ) {
        if !new_info.head.is_zero() {
            if old_info.head.is_zero() && new_info.open_block == new_info.head {
                self.cache.account_count.fetch_add(1, Ordering::SeqCst);
            }
            if !old_info.head.is_zero() && old_info.epoch != new_info.epoch {
                // store.account ().put won't erase existing entries if they're in different tables
                self.store.account().del(txn, account);
            }
            self.store.account().put(txn, account, new_info);
        } else {
            debug_assert!(!self.store.confirmation_height().exists(txn.txn(), account));
            self.store.account().del(txn, account);
            debug_assert!(self.cache.account_count.load(Ordering::SeqCst) > 0);
            self.cache.account_count.fetch_sub(1, Ordering::SeqCst);
        }
    }

    pub fn successor(&self, txn: &dyn Transaction, root: &QualifiedRoot) -> Option<BlockEnum> {
        let (mut successor, get_from_previous) = if root.previous.is_zero() {
            match self.store.account().get(txn, &root.root.into()) {
                Some(info) => (Some(info.open_block), false),
                None => (None, true),
            }
        } else {
            (None, true)
        };

        if get_from_previous {
            successor = self.store.block().successor(txn, &root.previous);
        }

        successor
            .map(|hash| self.store.block().get(txn, &hash))
            .flatten()
    }

    pub fn pruning_action(
        &self,
        txn: &mut dyn WriteTransaction,
        hash: &BlockHash,
        batch_size: u64,
    ) -> u64 {
        let mut pruned_count = 0;
        let mut hash = *hash;
        let genesis_hash = { self.constants.genesis.read().unwrap().hash() };

        while !hash.is_zero() && hash != genesis_hash {
            if let Some(block) = self.store.block().get(txn.txn(), &hash) {
                self.store.block().del(txn, &hash);
                self.store.pruned().put(txn, &hash);
                hash = block.previous();
                pruned_count += 1;
                self.cache.pruned_count.fetch_add(1, Ordering::SeqCst);
                if pruned_count % batch_size == 0 {
                    txn.commit();
                    txn.renew();
                }
            } else if self.store.pruned().exists(txn.txn(), &hash) {
                hash = BlockHash::zero();
            } else {
                panic!("Error finding block for pruning");
            }
        }

        pruned_count
    }

    /// **Warning:** In C++ the result is sorted in reverse order!
    pub fn unconfirmed_frontiers(&self) -> BTreeMap<u64, Vec<UncementedInfo>> {
        let result = Mutex::new(BTreeMap::<u64, Vec<UncementedInfo>>::new());
        self.store.account().for_each_par(&|txn, mut i, n| {
            let mut unconfirmed_frontiers = Vec::new();
            while !i.eq(n.as_ref()) {
                if let Some((&account, account_info)) = i.current() {
                    if let Some(conf_height_info) =
                        self.store.confirmation_height().get(txn.txn(), &account)
                    {
                        if account_info.block_count != conf_height_info.height {
                            // Always output as no confirmation height has been set on the account yet
                            let height_delta = account_info.block_count - conf_height_info.height;
                            let frontier = account_info.head;
                            let cemented_frontier = conf_height_info.frontier;
                            unconfirmed_frontiers.push((
                                height_delta,
                                UncementedInfo {
                                    cemented_frontier,
                                    frontier,
                                    account,
                                },
                            ))
                        }
                    }
                }
                i.next()
            }

            // Merge results
            let mut guard = result.lock().unwrap();
            for (delta, info) in unconfirmed_frontiers {
                guard.entry(delta).or_default().push(info);
            }
        });

        result.into_inner().unwrap()
    }

    pub fn bootstrap_weight_reached(&self) -> bool {
        self.cache.block_count.load(Ordering::SeqCst) >= self.bootstrap_weight_max_blocks()
    }

    pub fn write_confirmation_height(
        &self,
        txn: &mut dyn WriteTransaction,
        account: &Account,
        num_blocks_cemented: u64,
        confirmation_height: u64,
        confirmed_frontier: &BlockHash,
    ) {
        #[cfg(debug_assertions)]
        {
            let conf_height = self
                .store
                .confirmation_height()
                .get(txn.txn(), account)
                .map(|i| i.height)
                .unwrap_or_default();
            let block = self
                .store
                .block()
                .get(txn.txn(), confirmed_frontier)
                .unwrap();
            debug_assert!(block.sideband().unwrap().height == conf_height + num_blocks_cemented);
        }

        self.store.confirmation_height().put(
            txn,
            account,
            &ConfirmationHeightInfo::new(confirmation_height, *confirmed_frontier),
        );

        self.cache
            .cemented_count
            .fetch_add(num_blocks_cemented, Ordering::SeqCst);

        self.observer.blocks_cemented(num_blocks_cemented);
    }

    pub fn dependent_blocks(
        &self,
        txn: &dyn Transaction,
        block: &BlockEnum,
    ) -> (BlockHash, BlockHash) {
        DependentBlocksFinder::new(self, txn).find_dependent_blocks(block)
    }

    pub fn could_fit(&self, txn: &dyn Transaction, block: &BlockEnum) -> bool {
        let (first, second) = self.dependent_blocks(txn, block);
        self.is_dependency_satisfied(txn, &first) && self.is_dependency_satisfied(txn, &second)
    }

    fn is_dependency_satisfied(&self, txn: &dyn Transaction, dependency: &BlockHash) -> bool {
        dependency.is_zero() || self.store.block().exists(txn, dependency)
    }

    pub fn dependents_confirmed(&self, txn: &dyn Transaction, block: &BlockEnum) -> bool {
        let (first, second) = self.dependent_blocks(txn, block);
        self.is_dependency_confirmed(txn, &first) && self.is_dependency_confirmed(txn, &second)
    }

    fn is_dependency_confirmed(&self, txn: &dyn Transaction, dependency: &BlockHash) -> bool {
        if !dependency.is_zero() {
            self.block_confirmed(txn, dependency)
        } else {
            true
        }
    }

    /// Rollback blocks until `block' doesn't exist or it tries to penetrate the confirmation height
    pub fn rollback(
        &self,
        txn: &mut dyn WriteTransaction,
        block: &BlockHash,
    ) -> anyhow::Result<Vec<BlockEnum>> {
        BlockRollbackPerformer::new(self, txn).roll_back(block)
    }

    /// Returns the latest block with representative information
    pub fn representative_block_hash(&self, txn: &dyn Transaction, hash: &BlockHash) -> BlockHash {
        let hash = RepresentativeBlockFinder::new(txn, self.store.as_ref()).find_rep_block(*hash);
        debug_assert!(hash.is_zero() || self.store.block().exists(txn, &hash));
        hash
    }

    pub fn process(
        &self,
        txn: &mut dyn WriteTransaction,
        block: &mut BlockEnum,
    ) -> Result<(), ProcessResult> {
        let validator = BlockValidatorFactory::new(self, txn.txn(), block).create_validator();
        let instructions = validator.validate()?;
        BlockInserter::new(self, txn, block, &instructions).insert();
        Ok(())
    }

    pub fn get_block(&self, txn: &dyn Transaction, hash: &BlockHash) -> Option<BlockEnum> {
        self.store.block().get(txn, hash)
    }

    pub fn get_account_info(
        &self,
        transaction: &dyn Transaction,
        account: &Account,
    ) -> Option<AccountInfo> {
        self.store.account().get(transaction, account)
    }

    pub fn get_confirmation_height(
        &self,
        txn: &dyn Transaction,
        account: &Account,
    ) -> Option<ConfirmationHeightInfo> {
        self.store.confirmation_height().get(txn, account)
    }

    pub fn get_frontier(&self, txn: &dyn Transaction, hash: &BlockHash) -> Option<Account> {
        self.store.frontier().get(txn, hash)
    }

    pub fn get_pending(&self, txn: &dyn Transaction, key: &PendingKey) -> Option<PendingInfo> {
        self.store.pending().get(txn, key)
    }
}
