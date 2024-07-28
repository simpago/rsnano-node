use super::{
    election::{ElectionHandle, ElectionLockHandle},
    election_status::ElectionStatusHandle,
    recently_cemented_cache::{RecentlyCementedCachedDto, RecentlyCementedCachedRawData},
    vote_cache::VoteResultMapHandle,
    vote_with_weight_info::VoteWithWeightInfoVecHandle,
    VoteHandle,
};
use crate::{core::BlockHandle, utils::ContextWrapper, VoidPointerCallback};
use num_traits::FromPrimitive;
use rsnano_core::{Account, Amount, BlockEnum, BlockHash, QualifiedRoot, VoteSource};
use rsnano_node::{
    config::ActiveElectionsConfig,
    consensus::{ActiveElections, ActiveElectionsExt, Election, VoteApplierExt},
};
use std::{ffi::c_void, ops::Deref, sync::Arc};

pub struct ActiveTransactionsHandle(pub Arc<ActiveElections>);

impl Deref for ActiveTransactionsHandle {
    type Target = Arc<ActiveElections>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
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

#[no_mangle]
pub unsafe extern "C" fn rsn_active_transactions_add_election_winner_details(
    handle: &ActiveTransactionsHandle,
    hash: *const u8,
    election: &ElectionHandle,
) {
    handle
        .vote_applier
        .add_election_winner_details(BlockHash::from_ptr(hash), Arc::clone(election));
}

#[no_mangle]
pub unsafe extern "C" fn rsn_active_transactions_election_winner_details_len(
    handle: &ActiveTransactionsHandle,
) -> usize {
    handle.vote_applier.election_winner_details_len()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_active_transactions_tally_impl(
    handle: &ActiveTransactionsHandle,
    lock_handle: &mut ElectionLockHandle,
) -> *mut TallyBlocksHandle {
    let tally = handle
        .vote_applier
        .tally_impl(lock_handle.0.as_mut().unwrap());
    Box::into_raw(Box::new(TallyBlocksHandle(
        tally
            .iter()
            .map(|(key, value)| (key.deref().clone(), Arc::clone(value)))
            .collect(),
    )))
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
pub extern "C" fn rsn_active_transactions_active(
    handle: &ActiveTransactionsHandle,
    block: &BlockHandle,
) -> bool {
    handle.active(block)
}

#[no_mangle]
pub unsafe extern "C" fn rsn_active_transactions_len(handle: &ActiveTransactionsHandle) -> usize {
    handle.len()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_active_transactions_election(
    handle: &ActiveTransactionsHandle,
    root: *const u8,
) -> *mut ElectionHandle {
    handle
        .election(&QualifiedRoot::from_ptr(root))
        .map(ElectionHandle::new)
        .unwrap_or(std::ptr::null_mut())
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
    let result = handle
        .vote_router
        .vote(vote, VoteSource::from_u8(source).unwrap());
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
    handle.vote_applier.vote(
        election,
        &Account::from_ptr(rep),
        timestamp,
        &BlockHash::from_ptr(block_hash),
        VoteSource::from_u8(vote_source).unwrap(),
    ) as u8
}
/*
 * Callbacks
 */

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

#[repr(C)]
pub struct ActiveElectionsConfigDto {
    // Maximum number of simultaneous active elections (AEC size)
    pub size: usize,
    // Limit of hinted elections as percentage of `active_elections_size`
    pub hinted_limit_percentage: usize,
    // Limit of optimistic elections as percentage of `active_elections_size`
    pub optimistic_limit_percentage: usize,
    // Maximum confirmation history size
    pub confirmation_history_size: usize,
    // Maximum cache size for recently_confirmed
    pub confirmation_cache: usize,
}

impl From<&ActiveElectionsConfigDto> for ActiveElectionsConfig {
    fn from(value: &ActiveElectionsConfigDto) -> Self {
        Self {
            size: value.size,
            hinted_limit_percentage: value.hinted_limit_percentage,
            optimistic_limit_percentage: value.optimistic_limit_percentage,
            confirmation_history_size: value.confirmation_history_size,
            confirmation_cache: value.confirmation_cache,
        }
    }
}

impl From<&ActiveElectionsConfig> for ActiveElectionsConfigDto {
    fn from(value: &ActiveElectionsConfig) -> Self {
        Self {
            size: value.size,
            hinted_limit_percentage: value.hinted_limit_percentage,
            optimistic_limit_percentage: value.optimistic_limit_percentage,
            confirmation_history_size: value.confirmation_history_size,
            confirmation_cache: value.confirmation_cache,
        }
    }
}
