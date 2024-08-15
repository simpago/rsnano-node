use super::{
    wallet::{AccountVecHandle, WalletHandle},
    wallet_representatives::WalletRepresentativesLock,
};
use crate::{
    block_processing::BlockProcessorHandle,
    cementation::ConfirmingSetHandle,
    core::BlockHandle,
    ledger::datastore::{LedgerHandle, TransactionHandle},
    representatives::RepresentativeRegisterHandle,
    to_rust_string,
    transport::TcpChannelsHandle,
    utils::{ContextWrapper, ThreadPoolHandle},
    work::{DistributedWorkFactoryHandle, WorkThresholdsDto},
    NetworkParamsDto, NodeConfigDto, StatHandle, StringDto, VoidPointerCallback,
};
use rsnano_core::{
    work::WorkThresholds, Account, Amount, BlockEnum, BlockHash, RawKey, Root, WalletId,
};
use rsnano_node::{
    config::NodeConfig,
    representatives::OnlineReps,
    transport::MessagePublisher,
    wallets::{Wallet, Wallets, WalletsError, WalletsExt},
    NetworkParams,
};
use rsnano_store_lmdb::{EnvOptions, LmdbEnv, SyncStrategy};
use std::{
    collections::HashMap,
    ffi::{c_char, c_void, CStr},
    ops::Deref,
    path::PathBuf,
    sync::{Arc, Mutex, MutexGuard},
};

pub struct LmdbWalletsHandle(pub Arc<Wallets>);

impl Deref for LmdbWalletsHandle {
    type Target = Arc<Wallets>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_wallets_create(
    _enable_voting: bool,
    app_path: *const c_char,
    ledger: &LedgerHandle,
    node_config: &NodeConfigDto,
    kdf_work: u32,
    work_thresholds: &WorkThresholdsDto,
    distributed_work: &DistributedWorkFactoryHandle,
    network_params: &NetworkParamsDto,
    workers: &ThreadPoolHandle,
    block_processor: &BlockProcessorHandle,
    representatives: &RepresentativeRegisterHandle,
    tcp_channels: &TcpChannelsHandle,
    confirming_set: &ConfirmingSetHandle,
    stats: &StatHandle,
) -> *mut LmdbWalletsHandle {
    let node_config = NodeConfig::try_from(node_config).unwrap();

    let app_path = to_rust_string(app_path);
    let mut wallets_path = PathBuf::from(app_path);
    wallets_path.push("wallets.ldb");

    let mut lmdb_config = node_config.lmdb_config.clone();
    lmdb_config.sync = SyncStrategy::Always;
    lmdb_config.map_size = 1024 * 1024 * 1024;
    let options = EnvOptions {
        config: lmdb_config,
        use_no_mem_init: false,
    };
    let lmdb = Arc::new(LmdbEnv::new_with_options(wallets_path, &options).unwrap());

    let network_params = NetworkParams::try_from(network_params).unwrap();
    let protocol = network_params.network.protocol_info();
    let work = WorkThresholds::from(work_thresholds);
    let wallets = Arc::new(
        Wallets::new(
            lmdb,
            Arc::clone(&ledger.0),
            &node_config,
            kdf_work,
            work,
            Arc::clone(distributed_work),
            network_params,
            Arc::clone(workers),
            Arc::clone(block_processor),
            representatives.0.clone(),
            Arc::clone(confirming_set),
            MessagePublisher::new(
                Arc::new(Mutex::new(OnlineReps::default())),
                Arc::clone(tcp_channels),
                Arc::clone(stats),
                protocol,
            ),
        )
        .expect("could not create wallet"),
    );
    wallets.initialize2();
    Box::into_raw(Box::new(LmdbWalletsHandle(wallets)))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_wallets_destroy(handle: *mut LmdbWalletsHandle) {
    drop(Box::from_raw(handle))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_wallets_clear_send_ids(handle: *mut LmdbWalletsHandle) {
    (*handle).0.clear_send_ids()
}

pub struct WalletsMutexLockHandle(MutexGuard<'static, HashMap<WalletId, Arc<Wallet>>>);

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_wallets_mutex_lock(
    handle: &LmdbWalletsHandle,
) -> *mut WalletsMutexLockHandle {
    let guard = unsafe {
        let guard = handle.0.mutex.lock().unwrap();
        std::mem::transmute::<
            MutexGuard<'_, HashMap<WalletId, Arc<Wallet>>>,
            MutexGuard<'static, HashMap<WalletId, Arc<Wallet>>>,
        >(guard)
    };
    Box::into_raw(Box::new(WalletsMutexLockHandle(guard)))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_wallets_mutex_try_lock(
    handle: &LmdbWalletsHandle,
) -> *mut WalletsMutexLockHandle {
    match handle.0.mutex.try_lock() {
        Ok(guard) => {
            let guard = unsafe {
                std::mem::transmute::<
                    MutexGuard<'_, HashMap<WalletId, Arc<Wallet>>>,
                    MutexGuard<'static, HashMap<WalletId, Arc<Wallet>>>,
                >(guard)
            };
            Box::into_raw(Box::new(WalletsMutexLockHandle(guard)))
        }
        Err(_) => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_wallets_mutex_lock_destroy(handle: *mut WalletsMutexLockHandle) {
    drop(Box::from_raw(handle))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_wallets_mutex_lock_size(
    handle: &WalletsMutexLockHandle,
) -> usize {
    handle.0.len()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_wallets_mutex_lock_find(
    handle: &WalletsMutexLockHandle,
    wallet_id: *const u8,
    wallet: *mut *mut WalletHandle,
) -> bool {
    let wallet_id = WalletId::from_ptr(wallet_id);
    match handle.0.get(&wallet_id) {
        Some(w) => {
            *wallet = Box::into_raw(Box::new(WalletHandle(Arc::clone(w))));
            true
        }
        None => false,
    }
}

#[no_mangle]
pub extern "C" fn rsn_lmdb_wallets_mutex_lock_get_all(
    handle: &WalletsMutexLockHandle,
) -> *mut WalletVecHandle {
    let wallets = handle
        .0
        .iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    Box::into_raw(Box::new(WalletVecHandle(wallets)))
}

pub struct WalletVecHandle(Vec<(WalletId, Arc<Wallet>)>);

#[no_mangle]
pub extern "C" fn rsn_wallet_vec_len(handle: &WalletVecHandle) -> usize {
    handle.0.len()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_wallet_vec_get(
    handle: &WalletVecHandle,
    index: usize,
    wallet_id: *mut u8,
) -> *mut WalletHandle {
    let (id, wallet) = handle.0.get(index).unwrap();
    id.copy_bytes(wallet_id);
    Box::into_raw(Box::new(WalletHandle(Arc::clone(wallet))))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_wallet_vec_destroy(handle: *mut WalletVecHandle) {
    drop(Box::from_raw(handle));
}

pub type ForeachRepresentativeAction = extern "C" fn(*mut c_void, *const u8, *const u8);

#[no_mangle]
pub extern "C" fn rsn_wallets_foreach_representative(
    handle: &mut LmdbWalletsHandle,
    action: ForeachRepresentativeAction,
    action_context: *mut c_void,
    delete_context: VoidPointerCallback,
) {
    let context_wrapper = ContextWrapper::new(action_context, delete_context);
    handle.0.foreach_representative(move |keys| {
        let ctx = context_wrapper.get_context();
        action(
            ctx,
            keys.private_key().as_bytes().as_ptr(),
            keys.private_key().as_bytes().as_ptr(),
        );
    });
}

#[no_mangle]
pub unsafe extern "C" fn rsn_wallets_insert_watch(
    handle: &mut LmdbWalletsHandle,
    wallet_id: *const u8,
    accounts: &AccountVecHandle,
) -> u8 {
    match handle.insert_watch(&WalletId::from_ptr(wallet_id), &accounts) {
        Ok(()) => WalletsError::None as u8,
        Err(e) => e as u8,
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_wallets_attempt_password(
    handle: &mut LmdbWalletsHandle,
    wallet_id: *const u8,
    password: *const c_char,
) -> u8 {
    match handle.attempt_password(
        &WalletId::from_ptr(wallet_id),
        CStr::from_ptr(password).to_string_lossy(),
    ) {
        Ok(()) => WalletsError::None as u8,
        Err(e) => e as u8,
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_wallets_lock(
    handle: &mut LmdbWalletsHandle,
    wallet_id: *const u8,
) -> u8 {
    match handle.lock(&WalletId::from_ptr(wallet_id)) {
        Ok(()) => WalletsError::None as u8,
        Err(e) => e as u8,
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_wallets_valid_password(
    handle: &mut LmdbWalletsHandle,
    wallet_id: *const u8,
    valid: *mut bool,
) -> u8 {
    match handle.valid_password(&WalletId::from_ptr(wallet_id)) {
        Ok(val) => {
            *valid = val;
            WalletsError::None as u8
        }
        Err(e) => e as u8,
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_wallets_rekey(
    handle: &mut LmdbWalletsHandle,
    wallet_id: *const u8,
    password: *const c_char,
) -> u8 {
    let password = CStr::from_ptr(password).to_string_lossy();
    match handle.rekey(&WalletId::from_ptr(wallet_id), password) {
        Ok(()) => WalletsError::None as u8,
        Err(e) => e as u8,
    }
}

#[no_mangle]
pub extern "C" fn rsn_wallets_stop(handle: &LmdbWalletsHandle) {
    handle.stop();
}

#[no_mangle]
pub unsafe extern "C" fn rsn_wallets_get_delayed_work(
    handle: &LmdbWalletsHandle,
    account: *const u8,
    root: *mut u8,
) {
    handle
        .delayed_work
        .lock()
        .unwrap()
        .get(&Account::from_ptr(account))
        .cloned()
        .unwrap_or_default()
        .copy_bytes(root);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_wallets_representatives_lock(
    handle: &LmdbWalletsHandle,
) -> *mut WalletRepresentativesLock {
    let guard = handle.representative_wallets.lock().unwrap();
    WalletRepresentativesLock::new(guard)
}

#[no_mangle]
pub unsafe extern "C" fn rsn_wallets_insert_adhoc(
    handle: &LmdbWalletsHandle,
    wallet: &WalletHandle,
    key: *const u8,
    generate_work: bool,
    result: *mut u8,
) {
    let account = handle.insert_adhoc(wallet, &RawKey::from_ptr(key), generate_work);
    account.copy_bytes(result)
}

#[no_mangle]
pub unsafe extern "C" fn rsn_wallets_insert_adhoc2(
    handle: &LmdbWalletsHandle,
    wallet_id: *const u8,
    key: *const u8,
    generate_work: bool,
    result: *mut u8,
) -> u8 {
    match handle.insert_adhoc2(
        &WalletId::from_ptr(wallet_id),
        &RawKey::from_ptr(key),
        generate_work,
    ) {
        Ok(account) => {
            account.copy_bytes(result);
            WalletsError::None as u8
        }
        Err(e) => e as u8,
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_wallets_compute_reps(handle: &LmdbWalletsHandle) {
    handle.compute_reps();
}

#[no_mangle]
pub unsafe extern "C" fn rsn_wallets_destroy(handle: &LmdbWalletsHandle, wallet_id: *const u8) {
    handle.destroy(&WalletId::from_ptr(wallet_id));
}

#[no_mangle]
pub unsafe extern "C" fn rsn_wallets_exists(
    handle: &LmdbWalletsHandle,
    account: *const u8,
) -> bool {
    handle.exists(&Account::from_ptr(account))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_wallets_reload(handle: &LmdbWalletsHandle) {
    handle.reload();
}

#[no_mangle]
pub unsafe extern "C" fn rsn_wallets_remove_account(
    handle: &LmdbWalletsHandle,
    wallet_id: *const u8,
    account: *const u8,
) -> u8 {
    match handle.remove_account(&WalletId::from_ptr(wallet_id), &Account::from_ptr(account)) {
        Ok(_) => WalletsError::None as u8,
        Err(e) => e as u8,
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_wallets_work_set(
    handle: &LmdbWalletsHandle,
    wallet_id: *const u8,
    account: *const u8,
    work: u64,
) -> u8 {
    match handle.work_set(
        &WalletId::from_ptr(wallet_id),
        &Account::from_ptr(account),
        work,
    ) {
        Ok(_) => WalletsError::None as u8,
        Err(e) => e as u8,
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_wallets_move_accounts(
    handle: &LmdbWalletsHandle,
    source_id: *const u8,
    target_id: *const u8,
    accounts: &AccountVecHandle,
) -> i32 {
    match handle.move_accounts(
        &WalletId::from_ptr(source_id),
        &WalletId::from_ptr(target_id),
        accounts,
    ) {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_wallets_deterministic_index_get(
    handle: &LmdbWalletsHandle,
    wallet_id: *const u8,
    index: *mut u32,
) -> u8 {
    match handle.deterministic_index_get(&WalletId::from_ptr(wallet_id)) {
        Ok(i) => {
            *index = i;
            WalletsError::None as u8
        }
        Err(e) => e as u8,
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_wallets_deterministic_insert(
    handle: &LmdbWalletsHandle,
    wallet: &WalletHandle,
    tx: &mut TransactionHandle,
    generate_work: bool,
    key: *mut u8,
) {
    let k = handle.deterministic_insert(wallet, tx.as_write_txn(), generate_work);
    k.copy_bytes(key);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_wallets_deterministic_insert2(
    handle: &LmdbWalletsHandle,
    wallet_id: *const u8,
    index: u32,
    generate_work: bool,
    key: *mut u8,
) -> u8 {
    match handle.deterministic_insert_at(&WalletId::from_ptr(wallet_id), index, generate_work) {
        Ok(k) => {
            k.copy_bytes(key);
            WalletsError::None as u8
        }
        Err(e) => e as u8,
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_wallets_deterministic_insert3(
    handle: &LmdbWalletsHandle,
    wallet_id: *const u8,
    generate_work: bool,
    key: *mut u8,
) -> u8 {
    match handle.deterministic_insert2(&WalletId::from_ptr(wallet_id), generate_work) {
        Ok(k) => {
            k.copy_bytes(key);
            WalletsError::None as u8
        }
        Err(e) => e as u8,
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_wallets_change_seed(
    handle: &LmdbWalletsHandle,
    wallet: &WalletHandle,
    tx: &mut TransactionHandle,
    prv_key: *const u8,
    count: u32,
    pub_key: *mut u8,
) {
    let key =
        handle.change_seed_wallet(wallet, tx.as_write_txn(), &RawKey::from_ptr(prv_key), count);
    key.copy_bytes(pub_key);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_wallets_send_action(
    handle: &LmdbWalletsHandle,
    wallet: &WalletHandle,
    source: *const u8,
    account: *const u8,
    amount: *const u8,
    work: u64,
    generate_work: bool,
    id: *const c_char,
) -> *mut BlockHandle {
    let id = if id.is_null() {
        None
    } else {
        Some(CStr::from_ptr(id).to_str().unwrap().to_owned())
    };

    let block = handle.send_action(
        wallet,
        Account::from_ptr(source),
        Account::from_ptr(account),
        Amount::from_ptr(amount),
        work,
        generate_work,
        id,
    );

    match block {
        Some(b) => BlockHandle::new(Arc::new(b)),
        None => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_wallets_change_action(
    handle: &LmdbWalletsHandle,
    wallet: &WalletHandle,
    source: *const u8,
    representative: *const u8,
    work: u64,
    generate_work: bool,
) -> *mut BlockHandle {
    let block = handle.change_action(
        wallet,
        Account::from_ptr(source),
        Account::from_ptr(representative),
        work,
        generate_work,
    );

    match block {
        Some(b) => BlockHandle::new(Arc::new(b)),
        None => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_wallets_receive_action(
    handle: &LmdbWalletsHandle,
    wallet: &WalletHandle,
    send_hash: *const u8,
    representative: *const u8,
    amount: *const u8,
    account: *const u8,
    work: u64,
    generate_work: bool,
) -> *mut BlockHandle {
    let block = handle.receive_action(
        wallet,
        BlockHash::from_ptr(send_hash),
        Account::from_ptr(representative),
        Amount::from_ptr(amount),
        Account::from_ptr(account),
        work,
        generate_work,
    );

    match block {
        Some(b) => BlockHandle::new(Arc::new(b)),
        None => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_wallets_work_get(
    handle: &LmdbWalletsHandle,
    wallet_id: *const u8,
    account: *const u8,
) -> u64 {
    handle.work_get(&WalletId::from_ptr(wallet_id), &Account::from_ptr(account))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_wallets_work_get2(
    handle: &LmdbWalletsHandle,
    wallet_id: *const u8,
    account: *const u8,
    work: *mut u64,
) -> u8 {
    match handle.work_get2(&WalletId::from_ptr(wallet_id), &Account::from_ptr(account)) {
        Ok(w) => {
            *work = w;
            WalletsError::None as u8
        }
        Err(e) => e as u8,
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_wallets_get_accounts(
    handle: &LmdbWalletsHandle,
    max_results: usize,
) -> *mut AccountVecHandle {
    AccountVecHandle::new(handle.get_accounts(max_results))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_wallets_get_accounts_of_wallet(
    handle: &LmdbWalletsHandle,
    wallet_id: *const u8,
    result: &mut u8,
) -> *mut AccountVecHandle {
    match handle.get_accounts_of_wallet(&WalletId::from_ptr(wallet_id)) {
        Ok(accounts) => {
            *result = WalletsError::None as u8;
            AccountVecHandle::new(accounts)
        }
        Err(e) => {
            *result = e as u8;
            std::ptr::null_mut()
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_wallets_fetch(
    handle: &LmdbWalletsHandle,
    wallet_id: *const u8,
    account: *const u8,
    prv_key: *mut u8,
) -> u8 {
    match handle.fetch(&WalletId::from_ptr(wallet_id), &Account::from_ptr(account)) {
        Ok(key) => {
            key.copy_bytes(prv_key);
            WalletsError::None as u8
        }
        Err(e) => e as u8,
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_wallets_receive_sync(
    handle: &LmdbWalletsHandle,
    wallet: &WalletHandle,
    block: &BlockHandle,
    representative: *const u8,
    amount: *mut u8,
) -> bool {
    handle
        .receive_sync(
            Arc::clone(wallet),
            block,
            Account::from_ptr(representative),
            Amount::from_ptr(amount),
        )
        .is_err()
}

pub type WalletsStartElectionCallback = unsafe extern "C" fn(*mut c_void, *mut BlockHandle);

#[no_mangle]
pub extern "C" fn rsn_wallets_search_receivable(
    handle: &LmdbWalletsHandle,
    wallet: &WalletHandle,
    wallet_tx: &TransactionHandle,
) -> bool {
    handle
        .search_receivable(wallet, wallet_tx.as_txn())
        .is_err()
}

#[no_mangle]
pub extern "C" fn rsn_wallets_search_receivable_all(handle: &LmdbWalletsHandle) {
    handle.search_receivable_all();
}

#[no_mangle]
pub unsafe extern "C" fn rsn_wallets_search_receivable_wallet(
    handle: &LmdbWalletsHandle,
    wallet_id: *const u8,
) -> u8 {
    match handle.search_receivable_wallet(WalletId::from_ptr(wallet_id)) {
        Ok(_) => WalletsError::None as u8,
        Err(e) => e as u8,
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_wallets_enter_password(
    handle: &LmdbWalletsHandle,
    wallet: &WalletHandle,
    tx: &TransactionHandle,
    password: *const c_char,
) -> bool {
    handle
        .enter_password_wallet(
            wallet,
            tx.as_txn(),
            CStr::from_ptr(password).to_str().unwrap(),
        )
        .is_err()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_wallets_enter_password2(
    handle: &LmdbWalletsHandle,
    wallet_id: *const u8,
    password: *const c_char,
) -> u8 {
    match handle.enter_password(
        WalletId::from_ptr(wallet_id),
        CStr::from_ptr(password).to_str().unwrap(),
    ) {
        Ok(_) => WalletsError::None as u8,
        Err(e) => e as u8,
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_wallets_enter_initial_password(
    handle: &LmdbWalletsHandle,
    wallet: &WalletHandle,
) {
    handle.enter_initial_password(wallet);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_wallets_create(handle: &LmdbWalletsHandle, wallet_id: *const u8) {
    handle.create(WalletId::from_ptr(wallet_id));
}

#[no_mangle]
pub unsafe extern "C" fn rsn_wallets_change_sync_wallet(
    handle: &LmdbWalletsHandle,
    wallet: &WalletHandle,
    source: *const u8,
    representative: *const u8,
) -> bool {
    handle
        .change_sync_wallet(
            Arc::clone(wallet),
            Account::from_ptr(source),
            Account::from_ptr(representative),
        )
        .is_err()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_wallets_import(
    handle: &LmdbWalletsHandle,
    wallet_id: *const u8,
    json: *const c_char,
) -> bool {
    handle
        .import(
            WalletId::from_ptr(wallet_id),
            CStr::from_ptr(json).to_str().unwrap(),
        )
        .is_err()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_wallets_import_replace(
    handle: &LmdbWalletsHandle,
    wallet_id: *const u8,
    json: *const c_char,
    password: *const c_char,
) -> bool {
    handle
        .import_replace(
            WalletId::from_ptr(wallet_id),
            CStr::from_ptr(json).to_str().unwrap(),
            CStr::from_ptr(password).to_str().unwrap(),
        )
        .is_err()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_wallets_set_representative(
    handle: &LmdbWalletsHandle,
    wallet_id: *const u8,
    rep: *const u8,
    update_existing_accounts: bool,
) -> u8 {
    match handle.set_representative(
        WalletId::from_ptr(wallet_id),
        Account::from_ptr(rep),
        update_existing_accounts,
    ) {
        Ok(()) => WalletsError::None as u8,
        Err(e) => e as u8,
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_wallets_change_async(
    handle: &LmdbWalletsHandle,
    wallet_id: *const u8,
    source: *const u8,
    representative: *const u8,
    callback: WalletsStartElectionCallback,
    context: *mut c_void,
    delete_context: VoidPointerCallback,
    work: u64,
    generate_work: bool,
) -> u8 {
    let context_wrapper = ContextWrapper::new(context, delete_context);
    let callback_wrapper = Box::new(move |block: Option<BlockEnum>| {
        let block_handle = match block {
            Some(b) => BlockHandle::new(Arc::new(b)),
            None => std::ptr::null_mut(),
        };
        callback(context_wrapper.get_context(), block_handle);
    });
    match handle.change_async(
        WalletId::from_ptr(wallet_id),
        Account::from_ptr(source),
        Account::from_ptr(representative),
        callback_wrapper,
        work,
        generate_work,
    ) {
        Ok(()) => WalletsError::None as u8,
        Err(e) => e as u8,
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_wallets_ensure_wallet_is_unlocked(
    handle: &LmdbWalletsHandle,
    wallet_id: *const u8,
    password: *const c_char,
) -> bool {
    handle.ensure_wallet_is_unlocked(WalletId::from_ptr(wallet_id), &to_rust_string(password))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_wallets_change_seed2(
    handle: &LmdbWalletsHandle,
    wallet_id: *const u8,
    prv_key: *const u8,
    count: u32,
    first_account: *mut u8,
    restored_count: &mut u32,
) -> u8 {
    match handle.change_seed(
        WalletId::from_ptr(wallet_id),
        &RawKey::from_ptr(prv_key),
        count,
    ) {
        Ok((restored, first)) => {
            *restored_count = restored;
            first.copy_bytes(first_account);
            WalletsError::None as u8
        }
        Err(e) => e as u8,
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_wallets_get_seed(
    handle: &LmdbWalletsHandle,
    wallet_id: *const u8,
    prv_key: *mut u8,
) -> u8 {
    match handle.get_seed(WalletId::from_ptr(wallet_id)) {
        Ok(seed) => {
            seed.copy_bytes(prv_key);
            WalletsError::None as u8
        }
        Err(e) => e as u8,
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_wallets_receive_async(
    handle: &LmdbWalletsHandle,
    wallet_id: *const u8,
    hash: *const u8,
    representative: *const u8,
    amount: *const u8,
    account: *const u8,
    callback: WalletsStartElectionCallback,
    context: *mut c_void,
    delete_context: VoidPointerCallback,
    work: u64,
    generate_work: bool,
) -> u8 {
    let context_wrapper = ContextWrapper::new(context, delete_context);
    let callback_wrapper = Box::new(move |block: Option<BlockEnum>| {
        let block_handle = match block {
            Some(b) => BlockHandle::new(Arc::new(b)),
            None => std::ptr::null_mut(),
        };
        callback(context_wrapper.get_context(), block_handle);
    });
    match handle.receive_async(
        WalletId::from_ptr(wallet_id),
        BlockHash::from_ptr(hash),
        Account::from_ptr(representative),
        Amount::from_ptr(amount),
        Account::from_ptr(account),
        callback_wrapper,
        work,
        generate_work,
    ) {
        Ok(()) => WalletsError::None as u8,
        Err(e) => e as u8,
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_wallets_send_async(
    handle: &LmdbWalletsHandle,
    wallet_id: *const u8,
    source: *const u8,
    account: *const u8,
    amount: *const u8,
    callback: WalletsStartElectionCallback,
    context: *mut c_void,
    delete_context: VoidPointerCallback,
    work: u64,
    generate_work: bool,
    id: *const c_char,
) -> u8 {
    let context_wrapper = ContextWrapper::new(context, delete_context);
    let callback_wrapper = Box::new(move |block: Option<BlockEnum>| {
        let block_handle = match block {
            Some(b) => BlockHandle::new(Arc::new(b)),
            None => std::ptr::null_mut(),
        };
        callback(context_wrapper.get_context(), block_handle);
    });
    let id = if id.is_null() {
        None
    } else {
        Some(to_rust_string(id))
    };
    match handle.send_async(
        WalletId::from_ptr(wallet_id),
        Account::from_ptr(source),
        Account::from_ptr(account),
        Amount::from_ptr(amount),
        callback_wrapper,
        work,
        generate_work,
        id,
    ) {
        Ok(()) => WalletsError::None as u8,
        Err(e) => e as u8,
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_wallets_send_sync(
    handle: &LmdbWalletsHandle,
    wallet_id: *const u8,
    source: *const u8,
    account: *const u8,
    amount: *const u8,
    hash: *mut u8,
) {
    let result = handle.send_sync(
        WalletId::from_ptr(wallet_id),
        Account::from_ptr(source),
        Account::from_ptr(account),
        Amount::from_ptr(amount),
    );
    result.copy_bytes(hash);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_wallets_key_type(
    handle: &LmdbWalletsHandle,
    wallet_id: *const u8,
    account: *const u8,
) -> u8 {
    handle.key_type(WalletId::from_ptr(wallet_id), Account::from_ptr(account)) as u8
}

#[no_mangle]
pub unsafe extern "C" fn rsn_wallets_get_representative(
    handle: &LmdbWalletsHandle,
    wallet_id: *const u8,
    representative: *mut u8,
) -> u8 {
    match handle.get_representative(WalletId::from_ptr(wallet_id)) {
        Ok(rep) => {
            rep.copy_bytes(representative);
            WalletsError::None as u8
        }
        Err(e) => e as u8,
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_wallets_decrypt(
    handle: &LmdbWalletsHandle,
    wallet_id: *const u8,
    error: &mut u8,
) -> *mut DecryptResultHandle {
    match handle.decrypt(WalletId::from_ptr(wallet_id)) {
        Ok(decrytped) => {
            *error = WalletsError::None as u8;
            Box::into_raw(Box::new(DecryptResultHandle(decrytped)))
        }
        Err(e) => {
            *error = e as u8;
            std::ptr::null_mut()
        }
    }
}

pub struct DecryptResultHandle(Vec<(Account, RawKey)>);

#[no_mangle]
pub extern "C" fn rsn_decrypt_result_len(handle: &DecryptResultHandle) -> usize {
    handle.0.len()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_decrypt_result_get(
    handle: &DecryptResultHandle,
    index: usize,
    account: *mut u8,
    key: *mut u8,
) {
    handle.0[index].0.copy_bytes(account);
    handle.0[index].1.copy_bytes(key);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_decrypt_result_destroy(handle: *mut DecryptResultHandle) {
    drop(Box::from_raw(handle))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_wallets_serialize(
    handle: &LmdbWalletsHandle,
    wallet_id: *const u8,
    result: &mut StringDto,
) -> u8 {
    match handle.serialize(WalletId::from_ptr(wallet_id)) {
        Ok(json) => {
            *result = json.into();
            WalletsError::None as u8
        }
        Err(e) => e as u8,
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_wallets_work_cache_blocking(
    handle: &mut LmdbWalletsHandle,
    wallet_id: *const u8,
    account: *const u8,
    root: *const u8,
) {
    handle
        .work_cache_blocking2(
            &WalletId::from_ptr(wallet_id),
            &Account::from_ptr(account),
            &Root::from_ptr(root),
        )
        .unwrap();
}
