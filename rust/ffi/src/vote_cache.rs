use rsnano_node::vote_cache::{Config, VoteCache};

pub struct VoteCacheHandle(VoteCache);

#[no_mangle]
pub extern "C" fn rsn_vote_cache_create(max_size: usize) -> *mut VoteCacheHandle {
    Box::into_raw(Box::new(VoteCacheHandle(VoteCache::new(Config::new(
        max_size,
    )))))
}
