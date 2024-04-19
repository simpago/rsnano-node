use super::{
    election::{ElectionHandle, ElectionLockHandle},
    election_status::ElectionStatusHandle,
    recently_cemented_cache::{RecentlyCementedCachedDto, RecentlyCementedCachedRawData},
    vote_cache::{VoteCacheHandle, VoteResultMapHandle},
    vote_generator::VoteGeneratorHandle,
    vote_with_weight_info::VoteWithWeightInfoVecHandle,
    LocalVoteHistoryHandle, VoteHandle,
};
use crate::{
    block_processing::BlockProcessorHandle,
    cementation::ConfirmingSetHandle,
    core::{BlockHandle, BlockHashCallback},
    ledger::datastore::{lmdb::TransactionType, LedgerHandle, TransactionHandle},
    representatives::{OnlineRepsHandle, RepresentativeRegisterHandle},
    transport::TcpChannelsHandle,
    utils::{ContainerInfoComponentHandle, ContextWrapper, InstantHandle, ThreadPoolHandle},
    wallets::LmdbWalletsHandle,
    NetworkParamsDto, NodeConfigDto, StatHandle, VoidPointerCallback,
};
use num_traits::FromPrimitive;
use rsnano_core::{
    Account, Amount, BlockEnum, BlockHash, QualifiedRoot, Root, Vote, VoteCode, VoteSource,
};
use rsnano_node::{
    config::NodeConfig,
    consensus::{
        AccountBalanceChangedCallback, ActiveTransactions, ActiveTransactionsData,
        ActiveTransactionsExt, Election, ElectionBehavior, ElectionEndCallback,
    },
};
use std::{
    collections::HashMap,
    ffi::{c_char, c_void, CStr},
    ops::Deref,
    sync::{Arc, MutexGuard},
};

pub struct ActiveTransactionsHandle(Arc<ActiveTransactions>);

impl ActiveTransactionsHandle {
    pub fn new(inner: Arc<ActiveTransactions>) -> *mut Self {
        Box::into_raw(Box::new(Self(inner)))
    }
}

impl Deref for ActiveTransactionsHandle {
    type Target = Arc<ActiveTransactions>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_active_transactions_create(
    network: &NetworkParamsDto,
    online_reps: &OnlineRepsHandle,
    wallets: &LmdbWalletsHandle,
    config: &NodeConfigDto,
    ledger: &LedgerHandle,
    confirming_set: &ConfirmingSetHandle,
    workers: &ThreadPoolHandle,
    history: &LocalVoteHistoryHandle,
    block_processor: &BlockProcessorHandle,
    generator: &VoteGeneratorHandle,
    final_generator: &VoteGeneratorHandle,
    tcp_channels: &TcpChannelsHandle,
    vote_cache: &VoteCacheHandle,
    stats: &StatHandle,
    observers_context: *mut c_void,
    delete_observers_context: VoidPointerCallback,
    active_started: BlockHashCallback,
    active_stopped: BlockHashCallback,
    election_ended: ElectionEndedCallback,
    balance_changed: FfiAccountBalanceCallback,
    rep_register: &RepresentativeRegisterHandle,
) -> *mut ActiveTransactionsHandle {
    let ctx_wrapper = Arc::new(ContextWrapper::new(
        observers_context,
        delete_observers_context,
    ));

    let ctx = Arc::clone(&ctx_wrapper);
    let active_started_wrapper = Box::new(move |hash: BlockHash| {
        active_started(ctx.get_context(), hash.as_bytes().as_ptr())
    });

    let ctx = Arc::clone(&ctx_wrapper);
    let active_stopped_wrapper = Box::new(move |hash: BlockHash| {
        active_stopped(ctx.get_context(), hash.as_bytes().as_ptr())
    });

    let ctx = Arc::clone(&ctx_wrapper);
    let election_ended_wrapper: ElectionEndCallback = Box::new(
        move |status, votes, account, amount, is_state_send, is_state_epoch| {
            let status_handle = ElectionStatusHandle::new(status.clone());
            let votes_handle = VoteWithWeightInfoVecHandle::new(votes.clone());
            election_ended(
                ctx.get_context(),
                status_handle,
                votes_handle,
                account.as_bytes().as_ptr(),
                amount.to_be_bytes().as_ptr(),
                is_state_send,
                is_state_epoch,
            );
        },
    );

    let ctx = Arc::clone(&ctx_wrapper);
    let account_balance_changed_wrapper: AccountBalanceChangedCallback =
        Box::new(move |account, is_pending| {
            balance_changed(ctx.get_context(), account.as_bytes().as_ptr(), is_pending);
        });

    ActiveTransactionsHandle::new(Arc::new(ActiveTransactions::new(
        network.try_into().unwrap(),
        Arc::clone(online_reps),
        Arc::clone(wallets),
        NodeConfig::try_from(config).unwrap(),
        Arc::clone(ledger),
        Arc::clone(confirming_set),
        Arc::clone(workers),
        Arc::clone(history),
        Arc::clone(block_processor),
        Arc::clone(generator),
        Arc::clone(final_generator),
        Arc::clone(tcp_channels),
        Arc::clone(vote_cache),
        Arc::clone(stats),
        active_started_wrapper,
        active_stopped_wrapper,
        election_ended_wrapper,
        account_balance_changed_wrapper,
        Arc::clone(rep_register),
    )))
}

#[no_mangle]
pub extern "C" fn rsn_active_transactions_request_loop(handle: &ActiveTransactionsHandle) {
    handle.request_loop();
}

#[no_mangle]
pub extern "C" fn rsn_active_transactions_request_loop2(
    handle: &ActiveTransactionsHandle,
    lock_handle: &mut ActiveTransactionsLockHandle,
    stamp: &InstantHandle,
) {
    let guard = lock_handle.0.take().unwrap();
    let guard = handle.0.request_loop2(stamp.0, guard);
    lock_handle.0 = Some(guard);
}

#[no_mangle]
pub extern "C" fn rsn_active_transactions_notify_all(handle: &ActiveTransactionsHandle) {
    handle.condition.notify_all();
}

#[no_mangle]
pub unsafe extern "C" fn rsn_active_transactions_destroy(handle: *mut ActiveTransactionsHandle) {
    drop(Box::from_raw(handle))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_active_transactions_stop(handle: &ActiveTransactionsHandle) {
    handle.stop();
}

#[no_mangle]
pub unsafe extern "C" fn rsn_active_transactions_active_root(
    handle: &ActiveTransactionsHandle,
    root: *const u8,
) -> bool {
    let root = QualifiedRoot::from_ptr(root);
    handle.active_root(&root)
}

pub struct ActiveTransactionsLockHandle(Option<MutexGuard<'static, ActiveTransactionsData>>);

#[no_mangle]
pub extern "C" fn rsn_active_transactions_lock(
    handle: &ActiveTransactionsHandle,
) -> *mut ActiveTransactionsLockHandle {
    let guard = handle.0.mutex.lock().unwrap();
    let guard = unsafe {
        std::mem::transmute::<
            MutexGuard<ActiveTransactionsData>,
            MutexGuard<'static, ActiveTransactionsData>,
        >(guard)
    };
    Box::into_raw(Box::new(ActiveTransactionsLockHandle(Some(guard))))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_active_transactions_lock_destroy(
    handle: *mut ActiveTransactionsLockHandle,
) {
    drop(Box::from_raw(handle))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_active_transactions_lock_lock(
    handle: &mut ActiveTransactionsLockHandle,
    active_transactions: &ActiveTransactionsHandle,
) {
    let guard = active_transactions.0.mutex.lock().unwrap();
    let guard = unsafe {
        std::mem::transmute::<
            MutexGuard<ActiveTransactionsData>,
            MutexGuard<'static, ActiveTransactionsData>,
        >(guard)
    };
    handle.0 = Some(guard)
}

#[no_mangle]
pub unsafe extern "C" fn rsn_active_transactions_lock_unlock(
    handle: &mut ActiveTransactionsLockHandle,
) {
    drop(handle.0.take())
}

#[no_mangle]
pub extern "C" fn rsn_active_transactions_lock_owns_lock(
    handle: &ActiveTransactionsLockHandle,
) -> bool {
    handle.0.is_some()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_active_transactions_lock_stop(
    handle: &mut ActiveTransactionsLockHandle,
) {
    handle.0.as_mut().unwrap().stopped = true;
}

#[no_mangle]
pub unsafe extern "C" fn rsn_active_transactions_lock_roots_size(
    handle: &ActiveTransactionsLockHandle,
) -> usize {
    handle.0.as_ref().unwrap().roots.len()
}

#[no_mangle]
pub extern "C" fn rsn_active_transactions_lock_roots_clear(
    handle: &mut ActiveTransactionsLockHandle,
) {
    handle.0.as_mut().unwrap().roots.clear();
}

#[no_mangle]
pub unsafe extern "C" fn rsn_active_transactions_lock_roots_exists(
    handle: &ActiveTransactionsLockHandle,
    root: *const u8,
    previous: *const u8,
) -> bool {
    let root = QualifiedRoot::new(Root::from_ptr(root), BlockHash::from_ptr(previous));
    handle.0.as_ref().unwrap().roots.get(&root).is_some()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_active_transactions_lock_roots_find(
    handle: &ActiveTransactionsLockHandle,
    root: *const u8,
    previous: *const u8,
) -> *mut ElectionHandle {
    let root = QualifiedRoot::new(Root::from_ptr(root), BlockHash::from_ptr(previous));
    match handle.0.as_ref().unwrap().roots.get(&root) {
        Some(election) => Box::into_raw(Box::new(ElectionHandle(Arc::clone(election)))),
        None => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_active_transactions_lock_blocks_clear(
    handle: &mut ActiveTransactionsLockHandle,
) {
    handle.0.as_mut().unwrap().blocks.clear();
}

#[no_mangle]
pub unsafe extern "C" fn rsn_active_transactions_lock_blocks_find(
    handle: &ActiveTransactionsLockHandle,
    hash: *const u8,
) -> *mut ElectionHandle {
    let hash = BlockHash::from_ptr(hash);
    match handle.0.as_ref().unwrap().blocks.get(&hash) {
        Some(election) => Box::into_raw(Box::new(ElectionHandle(Arc::clone(election)))),
        None => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_active_transactions_tally_impl(
    handle: &ActiveTransactionsHandle,
    lock_handle: &mut ElectionLockHandle,
) -> *mut TallyBlocksHandle {
    let tally = handle.tally_impl(lock_handle.0.as_mut().unwrap());
    Box::into_raw(Box::new(TallyBlocksHandle(
        tally
            .iter()
            .map(|(key, value)| (key.deref().clone(), Arc::clone(value)))
            .collect(),
    )))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_active_transactions_remove_votes(
    handle: &ActiveTransactionsHandle,
    election: &ElectionHandle,
    lock_handle: &mut ElectionLockHandle,
    hash: *const u8,
) {
    handle.remove_votes(
        election,
        lock_handle.0.as_mut().unwrap(),
        &BlockHash::from_ptr(hash),
    );
}

#[no_mangle]
pub extern "C" fn rsn_active_transactions_list_active(
    handle: &ActiveTransactionsHandle,
    max: usize,
) -> *mut ElectionVecHandle {
    let elections = handle.0.list_active(max);
    Box::into_raw(Box::new(ElectionVecHandle(elections)))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_active_transactions_erase(
    handle: &ActiveTransactionsHandle,
    root: *const u8,
) -> bool {
    handle.erase(&QualifiedRoot::from_ptr(root))
}

#[no_mangle]
pub extern "C" fn rsn_active_transactions_publish_block(
    handle: &ActiveTransactionsHandle,
    block: &BlockHandle,
) -> bool {
    handle.publish_block(block)
}

pub struct TallyBlocksHandle(Vec<(Amount, Arc<BlockEnum>)>);

#[no_mangle]
pub unsafe extern "C" fn rsn_tally_blocks_create() -> *mut TallyBlocksHandle {
    Box::into_raw(Box::new(TallyBlocksHandle(Vec::new())))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_tally_blocks_destroy(handle: *mut TallyBlocksHandle) {
    drop(Box::from_raw(handle))
}

#[no_mangle]
pub extern "C" fn rsn_tally_blocks_len(handle: &TallyBlocksHandle) -> usize {
    handle.0.len()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_tally_blocks_get(
    handle: &TallyBlocksHandle,
    index: usize,
    weight: *mut u8,
) -> *mut BlockHandle {
    let (amount, block) = handle.0.get(index).unwrap();
    amount.copy_bytes(weight);
    BlockHandle::new(Arc::clone(block))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_tally_blocks_insert(
    handle: &mut TallyBlocksHandle,
    weight: *const u8,
    block: &BlockHandle,
) {
    handle.0.push((Amount::from_ptr(weight), Arc::clone(block)))
}

#[no_mangle]
pub extern "C" fn rsn_active_transactions_lock_roots_get_elections(
    handle: &ActiveTransactionsLockHandle,
) -> *mut ElectionVecHandle {
    let elections: Vec<_> = handle
        .0
        .as_ref()
        .unwrap()
        .roots
        .iter_sequenced()
        .map(|(_, election)| Arc::clone(election))
        .collect();

    Box::into_raw(Box::new(ElectionVecHandle(elections)))
}

pub struct ElectionVecHandle(Vec<Arc<Election>>);

#[no_mangle]
pub unsafe extern "C" fn rsn_election_vec_destroy(handle: *mut ElectionVecHandle) {
    drop(Box::from_raw(handle))
}

#[no_mangle]
pub extern "C" fn rsn_election_vec_len(handle: &ElectionVecHandle) -> usize {
    handle.0.len()
}

#[no_mangle]
pub extern "C" fn rsn_election_vec_get(
    handle: &ElectionVecHandle,
    index: usize,
) -> *mut ElectionHandle {
    Box::into_raw(Box::new(ElectionHandle(Arc::clone(&handle.0[index]))))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_active_transactions_remove_election_winner_details(
    handle: &ActiveTransactionsHandle,
    hash: *const u8,
) -> *mut ElectionHandle {
    match handle.remove_election_winner_details(&BlockHash::from_ptr(hash)) {
        Some(election) => ElectionHandle::new(election),
        None => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "C" fn rsn_active_transactions_confirm_once(
    handle: &ActiveTransactionsHandle,
    election_lock: &mut ElectionLockHandle,
    election: &ElectionHandle,
) {
    handle.confirm_once(election_lock.take().unwrap(), election);
}

#[no_mangle]
pub extern "C" fn rsn_active_transactions_process_confirmed(
    handle: &ActiveTransactionsHandle,
    status: &ElectionStatusHandle,
    iteration: u64,
) {
    handle.process_confirmed(status.0.clone(), iteration);
}

#[no_mangle]
pub extern "C" fn rsn_active_transactions_force_confirm(
    handle: &ActiveTransactionsHandle,
    election: &ElectionHandle,
) {
    handle.force_confirm(election);
}

#[no_mangle]
pub extern "C" fn rsn_active_transactions_confirmed(
    handle: &ActiveTransactionsHandle,
    election: &ElectionHandle,
) -> bool {
    handle.confirmed(election.0.as_ref())
}

#[no_mangle]
pub extern "C" fn rsn_active_transactions_limit(
    handle: &ActiveTransactionsHandle,
    behavior: u8,
) -> usize {
    handle.limit(FromPrimitive::from_u8(behavior).unwrap())
}

#[no_mangle]
pub extern "C" fn rsn_active_transactions_insert(
    handle: &ActiveTransactionsHandle,
    block: &BlockHandle,
    election_behavior: u8,
    inserted: &mut bool,
) -> *mut ElectionHandle {
    let (election_inserted, election) =
        handle.insert(block, ElectionBehavior::from_u8(election_behavior).unwrap());
    *inserted = election_inserted;
    election
        .map(|e| ElectionHandle::new(e))
        .unwrap_or(std::ptr::null_mut())
}

#[no_mangle]
pub extern "C" fn rsn_active_transactions_clear_recently_confirmed(
    handle: &ActiveTransactionsHandle,
) {
    handle.0.clear_recently_confirmed();
}

#[no_mangle]
pub extern "C" fn rsn_active_transactions_recently_confirmed_count(
    handle: &ActiveTransactionsHandle,
) -> usize {
    handle.0.recently_confirmed_count()
}

#[no_mangle]
pub extern "C" fn rsn_active_transactions_recently_cemented_count(
    handle: &ActiveTransactionsHandle,
) -> usize {
    handle.0.recently_cemented_count()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_active_transactions_was_recently_confirmed(
    handle: &ActiveTransactionsHandle,
    hash: *const u8,
) -> bool {
    handle.0.was_recently_confirmed(&BlockHash::from_ptr(hash))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_active_transactions_latest_recently_confirmed_root(
    handle: &ActiveTransactionsHandle,
    result: *mut u8,
) {
    handle
        .0
        .latest_recently_confirmed()
        .unwrap()
        .0
        .copy_bytes(result);
}

#[no_mangle]
pub extern "C" fn rsn_active_transactions_recently_confirmed_insert(
    handle: &ActiveTransactionsHandle,
    block: &BlockHandle,
) {
    handle.0.insert_recently_confirmed(block);
}

#[no_mangle]
pub extern "C" fn rsn_active_transactions_recently_cemented_insert(
    handle: &ActiveTransactionsHandle,
    status: &ElectionStatusHandle,
) {
    handle.0.insert_recently_cemented(status.0.clone());
}

#[no_mangle]
pub unsafe extern "C" fn rsn_active_transactions_recently_cemented_list(
    handle: &ActiveTransactionsHandle,
    list: *mut RecentlyCementedCachedDto,
) {
    let items: Vec<*mut ElectionStatusHandle> = handle
        .recently_cemented_list()
        .drain(..)
        .map(|e| Box::into_raw(Box::new(ElectionStatusHandle(e))))
        .collect();
    let raw_data = Box::into_raw(Box::new(RecentlyCementedCachedRawData(items)));
    (*list).items = (*raw_data).0.as_ptr();
    (*list).count = (*raw_data).0.len();
    (*list).raw_data = raw_data;
}

#[no_mangle]
pub extern "C" fn rsn_active_transactions_vacancy(
    handle: &ActiveTransactionsHandle,
    behavior: u8,
) -> i64 {
    handle.vacancy(FromPrimitive::from_u8(behavior).unwrap())
}

#[no_mangle]
pub extern "C" fn rsn_active_transactions_vacancy_update(handle: &ActiveTransactionsHandle) {
    (handle.vacancy_update.lock().unwrap())();
}

#[no_mangle]
pub extern "C" fn rsn_active_transactions_set_vacancy_update(
    handle: &ActiveTransactionsHandle,
    context: *mut c_void,
    callback: VoidPointerCallback,
    drop_context: VoidPointerCallback,
) {
    let ctx_wrapper = ContextWrapper::new(context, drop_context);
    *handle.vacancy_update.lock().unwrap() = Box::new(move || unsafe {
        callback(ctx_wrapper.get_context());
    })
}

#[no_mangle]
pub extern "C" fn rsn_active_transactions_clear(handle: &ActiveTransactionsHandle) {
    handle.clear();
}

#[no_mangle]
pub extern "C" fn rsn_active_transactions_confirmed_locked(
    handle: &ActiveTransactionsHandle,
    election: &ElectionLockHandle,
) -> bool {
    handle.confirmed_locked(&election.0.as_ref().unwrap())
}

#[no_mangle]
pub unsafe extern "C" fn rsn_active_transactions_try_confirm(
    handle: &ActiveTransactionsHandle,
    election: &ElectionHandle,
    hash: *const u8,
) {
    handle.try_confirm(election, &BlockHash::from_ptr(hash));
}

#[no_mangle]
pub unsafe extern "C" fn rsn_active_transactions_active(
    handle: &ActiveTransactionsHandle,
    hash: *const u8,
) -> bool {
    handle.active(&BlockHash::from_ptr(hash))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_active_transactions_publish(
    handle: &ActiveTransactionsHandle,
    block: &BlockHandle,
    election: &ElectionHandle,
) -> bool {
    handle.publish(block, election)
}

#[no_mangle]
pub unsafe extern "C" fn rsn_active_transactions_vote(
    handle: &ActiveTransactionsHandle,
    vote: &VoteHandle,
    source: u8,
) -> *mut VoteResultMapHandle {
    let result = handle.vote(vote, VoteSource::from_u8(source).unwrap());
    VoteResultMapHandle::new(&result)
}

#[no_mangle]
pub unsafe extern "C" fn rsn_active_transactions_vote2(
    handle: &ActiveTransactionsHandle,
    election: &ElectionHandle,
    rep: *const u8,
    timestamp: u64,
    block_hash: *const u8,
    vote_source: u8,
) -> u8 {
    handle.vote2(
        election,
        &Account::from_ptr(rep),
        timestamp,
        &BlockHash::from_ptr(block_hash),
        VoteSource::from_u8(vote_source).unwrap(),
    ) as u8
}

#[no_mangle]
pub unsafe extern "C" fn rsn_active_transactions_block_cemented_callback(
    handle: &ActiveTransactionsHandle,
    block: &BlockHandle,
) {
    handle.block_cemented_callback(block);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_active_transactions_collect_container_info(
    handle: &ActiveTransactionsHandle,
    name: *const c_char,
) -> *mut ContainerInfoComponentHandle {
    let container_info =
        handle.collect_container_info(CStr::from_ptr(name).to_str().unwrap().to_owned());
    Box::into_raw(Box::new(ContainerInfoComponentHandle(container_info)))
}

/*
 * Callbacks
 */
pub type VoteProcessedCallback =
    unsafe extern "C" fn(*mut c_void, *mut VoteHandle, u8, *mut VoteResultMapHandle);

#[no_mangle]
pub unsafe extern "C" fn rsn_active_transactions_add_vote_processed_observer(
    handle: &ActiveTransactionsHandle,
    observer: VoteProcessedCallback,
    context: *mut c_void,
    drop_context: VoidPointerCallback,
) {
    let ctx_wrapper = ContextWrapper::new(context, drop_context);
    let wrapped_observer = Box::new(
        move |vote: &Arc<Vote>, source: VoteSource, results: &HashMap<BlockHash, VoteCode>| {
            let vote_handle = VoteHandle::new(Arc::clone(vote));
            let results_handle = VoteResultMapHandle::new(results);
            observer(
                ctx_wrapper.get_context(),
                vote_handle,
                source as u8,
                results_handle,
            );
        },
    );
    handle.add_vote_processed_observer(wrapped_observer)
}

pub type ActivateSuccessorsCallback =
    unsafe extern "C" fn(*mut c_void, *mut TransactionHandle, *mut BlockHandle);

#[no_mangle]
pub unsafe extern "C" fn rsn_active_transactions_activate_successors(
    handle: &ActiveTransactionsHandle,
    callback: ActivateSuccessorsCallback,
    context: *mut c_void,
    drop_context: VoidPointerCallback,
) {
    let ctx_wrapper = ContextWrapper::new(context, drop_context);
    handle.set_activate_successors_callback(Box::new(move |tx, block| {
        let tx_handle = TransactionHandle::new(TransactionType::Read(tx));
        let block_handle = BlockHandle::new(Arc::clone(&block));
        callback(ctx_wrapper.get_context(), tx_handle, block_handle);
    }));
}

pub type ElectionEndedCallback = unsafe extern "C" fn(
    *mut c_void,
    *mut ElectionStatusHandle,
    *mut VoteWithWeightInfoVecHandle,
    *const u8,
    *const u8,
    bool,
    bool,
);

pub type FfiAccountBalanceCallback = unsafe extern "C" fn(*mut c_void, *const u8, bool);

//--------------------------------------------------------------------------------

pub struct ElectionWinnerDetailsLock(
    Option<MutexGuard<'static, HashMap<BlockHash, Arc<Election>>>>,
);

#[no_mangle]
pub unsafe extern "C" fn rsn_active_transactions_lock_election_winner_details(
    handle: &ActiveTransactionsHandle,
) -> *mut ElectionWinnerDetailsLock {
    let guard = handle.election_winner_details.lock().unwrap();
    let guard = std::mem::transmute::<
        MutexGuard<HashMap<BlockHash, Arc<Election>>>,
        MutexGuard<'static, HashMap<BlockHash, Arc<Election>>>,
    >(guard);
    Box::into_raw(Box::new(ElectionWinnerDetailsLock(Some(guard))))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_election_winner_details_lock_destroy(
    handle: *mut ElectionWinnerDetailsLock,
) {
    drop(Box::from_raw(handle))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_election_winner_details_lock_unlock(
    handle: &mut ElectionWinnerDetailsLock,
) {
    drop(handle.0.take())
}

#[no_mangle]
pub extern "C" fn rsn_election_winner_details_len(handle: &ElectionWinnerDetailsLock) -> usize {
    handle.0.as_ref().unwrap().len()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_election_winner_details_contains(
    handle: &ElectionWinnerDetailsLock,
    hash: *const u8,
) -> bool {
    handle
        .0
        .as_ref()
        .unwrap()
        .contains_key(&BlockHash::from_ptr(hash))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_election_winner_details_insert(
    handle: &mut ElectionWinnerDetailsLock,
    hash: *const u8,
    election: &ElectionHandle,
) {
    handle
        .0
        .as_mut()
        .unwrap()
        .insert(BlockHash::from_ptr(hash), Arc::clone(election));
}
