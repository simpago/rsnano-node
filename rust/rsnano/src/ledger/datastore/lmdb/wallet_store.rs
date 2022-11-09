use std::{
    fs::{set_permissions, File, Permissions},
    io::Write,
    os::unix::prelude::PermissionsExt,
    path::Path,
    sync::{Mutex, MutexGuard},
};

use lmdb::{Database, DatabaseFlags, WriteFlags};

use crate::{
    core::{deterministic_key, Account, PublicKey, RawKey},
    ledger::datastore::{iterator::DbIterator, Fans, Transaction, WalletValue, WriteTransaction},
    utils::{Deserialize, StreamAdapter},
    wallet::KeyDerivationFunction,
};

use super::{as_write_txn, get, LmdbIteratorImpl};

#[derive(FromPrimitive)]
pub enum KeyType {
    NotAType,
    Unknown,
    Adhoc,
    Deterministic,
}

const VERSION_CURRENT: u32 = 4;

pub type WalletIterator = Box<dyn DbIterator<Account, WalletValue>>;

pub struct LmdbWalletStore {
    db_handle: Mutex<Option<Database>>,
    pub fans: Mutex<Fans>,
    kdf: KeyDerivationFunction,
}

impl<'a> LmdbWalletStore {
    pub fn new(
        fanout: usize,
        kdf: KeyDerivationFunction,
        txn: &mut dyn WriteTransaction,
        representative: &Account,
        wallet: &Path,
    ) -> anyhow::Result<Self> {
        let store = Self {
            db_handle: Mutex::new(None),
            fans: Mutex::new(Fans::new(fanout)),
            kdf,
        };
        store.initialize(txn, wallet)?;
        let handle = store.db_handle();
        if let Err(lmdb::Error::NotFound) =
            get(txn.txn(), handle, Self::version_special().as_bytes())
        {
            store.version_put(txn, VERSION_CURRENT);
            let salt = RawKey::random();
            store.entry_put_raw(txn, &Self::salt_special(), &WalletValue::new(salt, 0));
            // Wallet key is a fixed random key that encrypts all entries
            let wallet_key = RawKey::random();
            let password = RawKey::zero();
            let mut guard = store.fans.lock().unwrap();
            guard.password.value_set(password);
            let zero = RawKey::zero();
            // Wallet key is encrypted by the user's password
            let encrypted = wallet_key.encrypt(&zero, &salt.initialization_vector_low());
            store.entry_put_raw(
                txn,
                &Self::wallet_key_special(),
                &WalletValue::new(encrypted, 0),
            );
            let wallet_key_enc = encrypted;
            guard.wallet_key_mem.value_set(wallet_key_enc);
            drop(guard);
            let check = zero.encrypt(&wallet_key, &salt.initialization_vector_low());
            store.entry_put_raw(txn, &Self::check_special(), &WalletValue::new(check, 0));
            let rep = RawKey::from_bytes(*representative.as_bytes());
            store.entry_put_raw(
                txn,
                &Self::representative_special(),
                &WalletValue::new(rep, 0),
            );
            let seed = RawKey::random();
            store.set_seed(txn, &seed);
            store.entry_put_raw(
                txn,
                &Self::deterministic_index_special(),
                &WalletValue::new(RawKey::zero(), 0),
            );
        }
        {
            let key = store
                .entry_get_raw(txn.txn(), &Self::wallet_key_special())
                .key;
            let mut guard = store.fans.lock().unwrap();
            guard.wallet_key_mem.value_set(key);
        }
        Ok(store)
    }

    pub fn new_from_json(
        fanout: usize,
        kdf: KeyDerivationFunction,
        txn: &mut dyn WriteTransaction,
        wallet: &Path,
        json: &str,
    ) -> anyhow::Result<Self> {
        let store = Self {
            db_handle: Mutex::new(None),
            fans: Mutex::new(Fans::new(fanout)),
            kdf,
        };
        store.initialize(txn, wallet)?;
        let handle = store.db_handle();
        match get(txn.txn(), handle, Self::version_special().as_bytes()) {
            Ok(_) => panic!("wallet store already initialized"),
            Err(lmdb::Error::NotFound) => {}
            Err(e) => panic!("unexpected wallet store error: {:?}", e),
        }

        let json: serde_json::Value = serde_json::from_str(json)?;
        if let serde_json::Value::Object(map) = json {
            for (k, v) in map.iter() {
                if let serde_json::Value::String(v_str) = v {
                    let key = Account::decode_hex(k)?;
                    let value = RawKey::decode_hex(v_str)?;
                    store.entry_put_raw(txn, &key, &WalletValue::new(value, 0));
                } else {
                    bail!("expected string value");
                }
            }
        } else {
            bail!("invalid json")
        }

        let tx = txn.txn();
        store.ensure_key_exists(tx, &Self::version_special())?;
        store.ensure_key_exists(tx, &Self::wallet_key_special())?;
        store.ensure_key_exists(tx, &Self::salt_special())?;
        store.ensure_key_exists(tx, &Self::check_special())?;
        store.ensure_key_exists(tx, &Self::representative_special())?;
        let mut guard = store.fans.lock().unwrap();
        guard.password.value_set(RawKey::zero());
        let key = store.entry_get_raw(tx, &Self::wallet_key_special()).key;
        guard.wallet_key_mem.value_set(key);
        drop(guard);
        Ok(store)
    }

    fn ensure_key_exists(&self, txn: &dyn Transaction, key: &Account) -> anyhow::Result<()> {
        get(txn, self.db_handle(), key.as_bytes())?;
        Ok(())
    }

    /// Wallet version number
    pub fn version_special() -> Account {
        Account::from(0)
    }

    /// Random number used to salt private key encryption
    pub fn salt_special() -> Account {
        Account::from(1)
    }

    /// Key used to encrypt wallet keys, encrypted itself by the user password
    pub fn wallet_key_special() -> Account {
        Account::from(2)
    }

    /// Check value used to see if password is valid
    pub fn check_special() -> Account {
        Account::from(3)
    }

    /// Representative account to be used if we open a new account
    pub fn representative_special() -> Account {
        Account::from(4)
    }

    /// Wallet seed for deterministic key generation
    pub fn seed_special() -> Account {
        Account::from(5)
    }

    /// Current key index for deterministic keys
    pub fn deterministic_index_special() -> Account {
        Account::from(6)
    }

    pub fn special_count() -> Account {
        Account::from(7)
    }

    pub fn initialize(&self, txn: &mut dyn WriteTransaction, path: &Path) -> anyhow::Result<()> {
        let path_str = path
            .as_os_str()
            .to_str()
            .ok_or_else(|| anyhow!("invalid path"))?;
        let db = unsafe { as_write_txn(txn).create_db(Some(path_str), DatabaseFlags::empty()) }?;
        *self.db_handle.lock().unwrap() = Some(db);
        Ok(())
    }

    pub fn db_handle(&self) -> Database {
        self.db_handle.lock().unwrap().unwrap()
    }

    pub fn entry_get_raw(&self, txn: &dyn Transaction, account: &Account) -> WalletValue {
        match get(txn, self.db_handle(), account.as_bytes()) {
            Ok(bytes) => {
                let mut stream = StreamAdapter::new(bytes);
                WalletValue::deserialize(&mut stream).unwrap()
            }
            _ => WalletValue::new(RawKey::zero(), 0),
        }
    }

    pub fn entry_put_raw(
        &self,
        txn: &mut dyn WriteTransaction,
        account: &Account,
        entry: &WalletValue,
    ) {
        as_write_txn(txn)
            .put(
                self.db_handle(),
                account.as_bytes(),
                &entry.to_bytes(),
                WriteFlags::empty(),
            )
            .unwrap();
    }

    pub fn check(&self, txn: &dyn Transaction) -> RawKey {
        self.entry_get_raw(txn, &Self::check_special()).key
    }

    pub fn salt(&self, txn: &dyn Transaction) -> RawKey {
        self.entry_get_raw(txn, &Self::salt_special()).key
    }

    pub fn wallet_key(&self, txn: &dyn Transaction) -> RawKey {
        let guard = self.fans.lock().unwrap();
        self.wallet_key_locked(&guard, txn)
    }

    fn wallet_key_locked(&self, guard: &MutexGuard<Fans>, txn: &dyn Transaction) -> RawKey {
        let wallet = guard.wallet_key_mem.value();
        let password = guard.password.value();
        let iv = self.salt(txn).initialization_vector_low();
        wallet.decrypt(&password, &iv)
    }

    pub fn seed(&self, txn: &dyn Transaction) -> RawKey {
        let value = self.entry_get_raw(txn, &Self::seed_special());
        let password = self.wallet_key(txn);
        let iv = self.salt(txn).initialization_vector_high();
        value.key.decrypt(&password, &iv)
    }

    pub fn set_seed(&self, txn: &mut dyn WriteTransaction, prv: &RawKey) {
        let password_l = self.wallet_key(txn.txn());
        let iv = self.salt(txn.txn()).initialization_vector_high();
        let ciphertext = prv.encrypt(&password_l, &iv);
        self.entry_put_raw(txn, &Self::seed_special(), &WalletValue::new(ciphertext, 0));
        self.deterministic_clear(txn);
    }

    pub fn deterministic_key(&self, txn: &dyn Transaction, index: u32) -> RawKey {
        debug_assert!(self.valid_password(txn));
        let seed = self.seed(txn);
        deterministic_key(&seed, index)
    }

    pub fn deterministic_index_get(&self, txn: &dyn Transaction) -> u32 {
        let value = self.entry_get_raw(txn, &Self::deterministic_index_special());
        value.key.number().low_u32()
    }

    pub fn deterministic_index_set(&self, txn: &mut dyn WriteTransaction, index: u32) {
        let index = RawKey::from(index as u64);
        let value = WalletValue::new(index, 0);
        self.entry_put_raw(txn, &Self::deterministic_index_special(), &value);
    }

    pub fn valid_password(&self, txn: &dyn Transaction) -> bool {
        let wallet_key = self.wallet_key(txn);
        self.check_wallet_key(txn, &wallet_key)
    }

    pub fn valid_password_locked(&self, guard: &MutexGuard<Fans>, txn: &dyn Transaction) -> bool {
        let wallet_key = self.wallet_key_locked(guard, txn);
        self.check_wallet_key(txn, &wallet_key)
    }

    fn check_wallet_key(&self, txn: &dyn Transaction, wallet_key: &RawKey) -> bool {
        let zero = RawKey::zero();
        let iv = self.salt(txn).initialization_vector_low();
        let check = zero.encrypt(&wallet_key, &iv);
        self.check(txn) == check
    }

    pub fn derive_key(&self, txn: &dyn Transaction, password: &str) -> RawKey {
        let salt = self.salt(txn);
        self.kdf.hash_password(password, salt.as_bytes())
    }

    pub fn rekey(&self, txn: &mut dyn WriteTransaction, password: &str) -> anyhow::Result<()> {
        let mut guard = self.fans.lock().unwrap();
        let tx = txn.txn();
        if self.valid_password_locked(&guard, tx) {
            let password_new = self.derive_key(tx, password);
            let wallet_key = self.wallet_key_locked(&guard, tx);
            guard.password.value_set(password_new);
            let iv = self.salt(tx).initialization_vector_low();
            let encrypted = wallet_key.encrypt(&password_new, &iv);
            guard.wallet_key_mem.value_set(encrypted);
            self.entry_put_raw(
                txn,
                &Self::wallet_key_special(),
                &WalletValue::new(encrypted, 0),
            );
            Ok(())
        } else {
            Err(anyhow!("invalid password"))
        }
    }

    pub fn begin(&self, txn: &dyn Transaction) -> WalletIterator {
        LmdbIteratorImpl::new_iterator(
            txn,
            self.db_handle(),
            Some(Self::special_count().as_bytes()),
            true,
        )
    }

    pub fn begin_at_account(&self, txn: &dyn Transaction, key: &Account) -> WalletIterator {
        LmdbIteratorImpl::new_iterator(txn, self.db_handle(), Some(key.as_bytes()), true)
    }

    pub fn end(&self) -> WalletIterator {
        LmdbIteratorImpl::null_iterator()
    }

    pub fn find(&self, txn: &dyn Transaction, account: &Account) -> WalletIterator {
        let result = self.begin_at_account(txn, account);
        if let Some((key, _)) = result.current() {
            if key == account {
                return result;
            }
        }

        self.end()
    }

    pub fn erase(&self, txn: &mut dyn WriteTransaction, account: &Account) {
        as_write_txn(txn)
            .del(self.db_handle(), account.as_bytes(), None)
            .unwrap();
    }

    pub fn key_type(value: &WalletValue) -> KeyType {
        let number = value.key.number();
        if number > u64::MAX.into() {
            KeyType::Adhoc
        } else {
            if (number >> 32).low_u32() == 1 {
                KeyType::Deterministic
            } else {
                KeyType::Unknown
            }
        }
    }

    pub fn deterministic_clear(&self, txn: &mut dyn WriteTransaction) {
        let mut it = self.begin(txn.txn());
        while let Some((account, value)) = it.current() {
            match Self::key_type(value) {
                KeyType::Deterministic => {
                    self.erase(txn, account);
                    it = self.begin_at_account(txn.txn(), account);
                }
                _ => it.next(),
            }
        }

        self.deterministic_index_set(txn, 0);
    }

    pub fn valid_public_key(&self, key: &PublicKey) -> bool {
        key.number() >= Self::special_count().number()
    }

    pub fn exists(&self, txn: &dyn Transaction, key: &PublicKey) -> bool {
        self.valid_public_key(key) && !self.find(txn, &Account::from(key)).is_end()
    }

    pub fn deterministic_insert(&self, txn: &mut dyn WriteTransaction) -> PublicKey {
        let tx = txn.txn();
        let mut index = self.deterministic_index_get(tx);
        let mut prv = self.deterministic_key(tx, index);
        let mut result = PublicKey::try_from(&prv).unwrap();
        while self.exists(tx, &result) {
            index += 1;
            prv = self.deterministic_key(tx, index);
            result = PublicKey::try_from(&prv).unwrap();
        }

        let mut marker = 1u64;
        marker <<= 32;
        marker |= index as u64;
        self.entry_put_raw(
            txn,
            &Account::from(result),
            &WalletValue::new(marker.into(), 0),
        );
        index += 1;
        self.deterministic_index_set(txn, index);
        return result;
    }

    pub fn deterministic_insert_at(&self, txn: &mut dyn WriteTransaction, index: u32) -> PublicKey {
        let prv = self.deterministic_key(txn.txn(), index);
        let result = PublicKey::try_from(&prv).unwrap();
        let mut marker = 1u64;
        marker <<= 32;
        marker |= index as u64;
        self.entry_put_raw(txn, &result.into(), &&WalletValue::new(marker.into(), 0));
        result
    }

    pub fn version(&self, txn: &dyn Transaction) -> u32 {
        let value = self.entry_get_raw(txn, &Self::version_special());
        value.key.as_bytes()[31] as u32
    }

    pub fn attempt_password(&self, txn: &dyn Transaction, password: &str) -> bool {
        let is_valid = {
            let mut guard = self.fans.lock().unwrap();
            let password_key = self.derive_key(txn, password);
            guard.password.value_set(password_key);
            self.valid_password_locked(&guard, txn)
        };

        if is_valid {
            if self.version(txn) != 4 {
                panic!("invalid wallet store version!");
            }
        }

        is_valid
    }

    pub fn lock(&self) {
        self.fans.lock().unwrap().password.value_set(RawKey::zero());
    }

    pub fn accounts(&self, txn: &dyn Transaction) -> Vec<Account> {
        let mut result = Vec::new();
        let mut it = self.begin(txn);
        while let Some((k, _)) = it.current() {
            result.push(*k);
            it.next();
        }

        result
    }

    pub fn representative(&self, txn: &dyn Transaction) -> Account {
        let value = self.entry_get_raw(txn, &Self::representative_special());
        Account::from_bytes(*value.key.as_bytes())
    }

    pub fn representative_set(&self, txn: &mut dyn WriteTransaction, representative: &Account) {
        let rep = RawKey::from_bytes(*representative.as_bytes());
        self.entry_put_raw(
            txn,
            &Self::representative_special(),
            &WalletValue::new(rep, 0),
        );
    }

    pub fn insert_adhoc(&self, txn: &mut dyn WriteTransaction, prv: &RawKey) -> PublicKey {
        debug_assert!(self.valid_password(txn.txn()));
        let pub_key = PublicKey::try_from(prv).unwrap();
        let password = self.wallet_key(txn.txn());
        let ciphertext = prv.encrypt(&password, &pub_key.initialization_vector());
        self.entry_put_raw(txn, &pub_key.into(), &WalletValue::new(ciphertext, 0));
        pub_key
    }

    pub fn insert_watch(
        &self,
        txn: &mut dyn WriteTransaction,
        pub_key: &Account,
    ) -> anyhow::Result<()> {
        if !self.valid_public_key(&pub_key.into()) {
            bail!("invalid public key");
        }

        self.entry_put_raw(txn, pub_key, &WalletValue::new(RawKey::zero(), 0));
        Ok(())
    }

    pub fn fetch(&self, txn: &dyn Transaction, pub_key: &Account) -> anyhow::Result<RawKey> {
        if !self.valid_password(txn) {
            bail!("invalid password");
        }

        let value = self.entry_get_raw(txn, pub_key);
        if value.key.is_zero() {
            bail!("pub key not found");
        }

        let prv = match Self::key_type(&value) {
            KeyType::Deterministic => {
                let index = value.key.number().low_u32();
                self.deterministic_key(txn, index)
            }
            KeyType::Adhoc => {
                // Ad-hoc keys
                let password = self.wallet_key(txn);
                let pub_key = PublicKey::from(pub_key);
                value
                    .key
                    .decrypt(&password, &pub_key.initialization_vector())
            }
            _ => bail!("invalid key type"),
        };

        let compare = PublicKey::try_from(&prv)?;
        if compare != pub_key.into() {
            bail!("expected pub key does not match");
        }
        Ok(prv)
    }

    pub fn serialize_json(&self, txn: &dyn Transaction) -> String {
        let mut map = serde_json::Map::new();
        let mut it = LmdbIteratorImpl::new_iterator::<Account, WalletValue>(
            txn,
            self.db_handle(),
            None,
            true,
        );

        while let Some((k, v)) = it.current() {
            map.insert(
                k.encode_hex(),
                serde_json::Value::String(v.key.encode_hex()),
            );
            it.next();
        }

        serde_json::Value::Object(map).to_string()
    }

    pub fn write_backup(&self, txn: &dyn Transaction, path: &Path) -> anyhow::Result<()> {
        let mut file = File::create(path)?;
        set_permissions(path, Permissions::from_mode(0o600))?;
        write!(file, "{}", self.serialize_json(txn))?;
        Ok(())
    }

    pub fn move_keys(
        &self,
        txn: &mut dyn WriteTransaction,
        other: &LmdbWalletStore,
        keys: &[PublicKey],
    ) -> anyhow::Result<()> {
        debug_assert!(self.valid_password(txn.txn()));
        debug_assert!(other.valid_password(txn.txn()));
        for k in keys {
            let prv = other.fetch(txn.txn(), &k.into())?;
            self.insert_adhoc(txn, &prv);
            other.erase(txn, &k.into());
        }

        Ok(())
    }

    pub fn import(
        &self,
        txn: &mut dyn WriteTransaction,
        other: &LmdbWalletStore,
    ) -> anyhow::Result<()> {
        debug_assert!(self.valid_password(txn.txn()));
        debug_assert!(other.valid_password(txn.txn()));
        let mut it = other.begin(txn.txn());
        while let Some((k, _)) = it.current() {
            let prv = other.fetch(txn.txn(), k)?;
            if !prv.is_zero() {
                self.insert_adhoc(txn, &prv);
            } else {
                self.insert_watch(txn, k)?;
            }
            other.erase(txn, k);

            it.next();
        }

        Ok(())
    }

    pub fn work_get(&self, txn: &dyn Transaction, pub_key: &PublicKey) -> anyhow::Result<u64> {
        let entry = self.entry_get_raw(txn, &pub_key.into());
        if !entry.key.is_zero() {
            Ok(entry.work)
        } else {
            Err(anyhow!("not found"))
        }
    }

    pub fn version_put(&self, txn: &mut dyn WriteTransaction, version: u32) {
        let entry = RawKey::from(version as u64);
        self.entry_put_raw(txn, &Self::version_special(), &WalletValue::new(entry, 0));
    }

    pub fn work_put(&self, txn: &mut dyn WriteTransaction, pub_key: &PublicKey, work: u64) {
        let mut entry = self.entry_get_raw(txn.txn(), &pub_key.into());
        debug_assert!(!entry.key.is_zero());
        entry.work = work;
        self.entry_put_raw(txn, &pub_key.into(), &entry);
    }

    pub fn destroy(&self, txn: &mut dyn WriteTransaction) {
        unsafe {
            as_write_txn(txn).drop_db(self.db_handle()).unwrap();
        }
        *self.db_handle.lock().unwrap() = None;
    }

    pub fn is_open(&self) -> bool {
        self.db_handle.lock().unwrap().is_some()
    }
}
