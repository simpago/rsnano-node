use rsnano_core::{Account, HashOrAccount};
use std::{
    collections::{HashMap, VecDeque},
    mem::size_of,
    time::Instant,
};

#[derive(Default, PartialEq, Eq, Debug, Clone)]
pub(crate) enum QueryType {
    #[default]
    Invalid,
    BlocksByHash,
    BlocksByAccount,
    // TODO: account_info
}

#[derive(Clone)]
pub(crate) struct AsyncTag {
    pub query_type: QueryType,
    pub id: u64,
    pub start: HashOrAccount,
    pub time: Instant,
    pub account: Account,
}

#[derive(Default)]
pub(crate) struct OrderedTags {
    by_id: HashMap<u64, AsyncTag>,
    by_account: HashMap<Account, Vec<u64>>,
    sequenced: VecDeque<u64>,
}

impl OrderedTags {
    pub const ELEMENT_SIZE: usize =
        size_of::<AsyncTag>() + size_of::<Account>() + size_of::<u64>() * 3;

    pub(crate) fn len(&self) -> usize {
        self.sequenced.len()
    }

    pub fn contains(&self, id: u64) -> bool {
        self.by_id.contains_key(&id)
    }

    #[allow(dead_code)]
    pub fn get(&self, id: u64) -> Option<&AsyncTag> {
        self.by_id.get(&id)
    }

    pub fn remove(&mut self, id: u64) -> Option<AsyncTag> {
        if let Some(tag) = self.by_id.remove(&id) {
            self.remove_by_account(id, &tag.account);
            self.sequenced.retain(|i| *i != id);
            Some(tag)
        } else {
            None
        }
    }

    pub fn front(&self) -> Option<&AsyncTag> {
        self.sequenced.front().map(|id| self.by_id.get(id).unwrap())
    }

    pub fn pop_front(&mut self) -> Option<AsyncTag> {
        if let Some(id) = self.sequenced.pop_front() {
            let result = self.by_id.remove(&id).unwrap();
            self.remove_by_account(id, &result.account);
            Some(result)
        } else {
            None
        }
    }

    pub(crate) fn insert(&mut self, tag: AsyncTag) {
        let id = tag.id;
        let account = tag.account;
        if let Some(old) = self.by_id.insert(id, tag) {
            self.remove_internal(old.id, &old.account);
        }
        self.by_account.entry(account).or_default().push(id);
        self.sequenced.push_back(id);
    }

    fn remove_internal(&mut self, id: u64, account: &Account) {
        self.by_id.remove(&id);
        self.remove_by_account(id, account);
        self.sequenced.retain(|i| *i != id);
    }

    fn remove_by_account(&mut self, id: u64, account: &Account) {
        if let Some(ids) = self.by_account.get_mut(account) {
            if ids.len() == 1 {
                self.by_account.remove(account);
            } else {
                ids.retain(|i| *i != id)
            }
        }
    }
}
