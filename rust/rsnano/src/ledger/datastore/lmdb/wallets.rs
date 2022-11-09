use super::{as_write_txn, get, LmdbEnv, LmdbIteratorImpl};
use crate::{
    core::{BlockHash, NoValue, RawKey, WalletId},
    ledger::datastore::{iterator::DbIterator, BinaryDbIterator, WriteTransaction},
};
use lmdb::{Cursor, Database, DatabaseFlags, Transaction, WriteFlags};
pub type WalletsIterator = BinaryDbIterator<[u8; 64], NoValue, LmdbIteratorImpl>;

pub struct LmdbWallets {
    pub handle: Option<Database>,
    pub send_action_ids_handle: Option<Database>,
}

impl LmdbWallets {
    pub fn new() -> Self {
        Self {
            handle: None,
            send_action_ids_handle: None,
        }
    }

    pub fn initialize(
        &mut self,
        txn: &mut dyn WriteTransaction,
        env: &LmdbEnv,
    ) -> anyhow::Result<()> {
        self.handle = Some(unsafe { as_write_txn(txn).create_db(None, DatabaseFlags::empty())? });
        self.split_if_needed(txn, env)?;
        self.send_action_ids_handle = Some(unsafe {
            as_write_txn(txn).create_db(Some("send_action_ids"), DatabaseFlags::empty())?
        });
        Ok(())
    }

    pub fn get_store_it(&self, txn: &dyn super::super::Transaction, hash: &str) -> WalletsIterator {
        let hash_bytes: [u8; 64] = hash.as_bytes().try_into().unwrap();
        WalletsIterator::new(LmdbIteratorImpl::new(
            txn,
            self.handle.unwrap(),
            Some(&hash_bytes),
            true,
        ))
    }

    pub fn move_table(
        &self,
        name: &str,
        txn_source: &mut dyn WriteTransaction,
        txn_destination: &mut dyn WriteTransaction,
    ) -> anyhow::Result<()> {
        let rw_txn_source = as_write_txn(txn_source);
        let rw_txn_dest = as_write_txn(txn_destination);
        let handle_source = unsafe { rw_txn_source.create_db(Some(name), DatabaseFlags::empty()) }?;
        let handle_destination =
            unsafe { rw_txn_dest.create_db(Some(name), DatabaseFlags::empty()) }?;

        {
            let mut cursor = rw_txn_source.open_ro_cursor(handle_source)?;
            for x in cursor.iter_start() {
                let (k, v) = x?;
                rw_txn_dest.put(handle_destination, &k, &v, WriteFlags::empty())?;
            }
        }

        unsafe { rw_txn_source.drop_db(handle_source) }?;
        Ok(())
    }

    pub fn split_if_needed(
        &self,
        txn_destination: &mut dyn WriteTransaction,
        env: &LmdbEnv,
    ) -> anyhow::Result<()> {
        let beginning = RawKey::from(0).encode_hex();
        let end = RawKey::from_bytes([1; 32]).encode_hex();

        // First do a read pass to check if there are any wallets that need extracting (to save holding a write lock and potentially being blocked)
        let wallets_need_splitting = {
            let transaction_source = env.tx_begin_read()?;
            let i = self.get_store_it(&transaction_source, &beginning);
            let n = self.get_store_it(&transaction_source, &end);
            i.current().map(|(k, _)| *k) != n.current().map(|(k, _)| *k)
        };

        if wallets_need_splitting {
            let mut txn_source = env.tx_begin_write().unwrap();
            let mut i = self.get_store_it(&txn_source, &beginning);
            while let Some((k, _)) = i.current() {
                let text = std::str::from_utf8(k)?;
                let _id = WalletId::decode_hex(text)?;
                self.move_table(text, &mut txn_source, txn_destination)?;
                i.next();
            }
        }
        Ok(())
    }

    pub fn get_wallet_ids(&self, txn: &dyn super::super::Transaction) -> Vec<WalletId> {
        let mut wallet_ids = Vec::new();
        let beginning = RawKey::from(0).encode_hex();
        let mut i = self.get_store_it(txn, &beginning);
        while let Some((k, _)) = i.current() {
            let text = std::str::from_utf8(k).unwrap();
            wallet_ids.push(WalletId::decode_hex(&text).unwrap());
            i.next();
        }
        wallet_ids
    }

    pub fn get_block_hash(
        &self,
        txn: &dyn super::super::Transaction,
        id: &str,
    ) -> anyhow::Result<Option<BlockHash>> {
        match get(txn, self.send_action_ids_handle.unwrap(), &id.as_bytes()) {
            Ok(bytes) => Ok(Some(
                BlockHash::from_slice(bytes).ok_or_else(|| anyhow!("invalid block hash"))?,
            )),
            Err(lmdb::Error::NotFound) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn set_block_hash(
        &self,
        txn: &mut dyn WriteTransaction,
        id: &str,
        hash: &BlockHash,
    ) -> anyhow::Result<()> {
        as_write_txn(txn).put(
            self.send_action_ids_handle.unwrap(),
            &id.as_bytes(),
            hash.as_bytes(),
            WriteFlags::empty(),
        )?;
        Ok(())
    }

    pub fn clear_send_ids(&self, txn: &mut dyn WriteTransaction) {
        as_write_txn(txn)
            .clear_db(self.send_action_ids_handle.unwrap())
            .unwrap();
    }
}
