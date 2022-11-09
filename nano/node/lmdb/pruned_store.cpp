#include <nano/node/lmdb/lmdb.hpp>
#include <nano/node/lmdb/pruned_store.hpp>

namespace
{
nano::store_iterator<nano::block_hash, std::nullptr_t> to_iterator (rsnano::LmdbIteratorHandle * it_handle)
{
	if (it_handle == nullptr)
	{
		return { nullptr };
	}

	return { std::make_unique<nano::mdb_iterator<nano::block_hash, std::nullptr_t>> (it_handle) };
}
}

nano::lmdb::pruned_store::pruned_store (rsnano::LmdbPrunedStoreHandle * handle_a) :
	handle{ handle_a }
{
}

nano::lmdb::pruned_store::~pruned_store ()
{
	if (handle != nullptr)
		rsnano::rsn_lmdb_pruned_store_destroy (handle);
}

void nano::lmdb::pruned_store::put (nano::write_transaction const & transaction_a, nano::block_hash const & hash_a)
{
	rsnano::rsn_lmdb_pruned_store_put (handle, transaction_a.get_rust_handle (), hash_a.bytes.data ());
}

void nano::lmdb::pruned_store::del (nano::write_transaction const & transaction_a, nano::block_hash const & hash_a)
{
	rsnano::rsn_lmdb_pruned_store_del (handle, transaction_a.get_rust_handle (), hash_a.bytes.data ());
}

bool nano::lmdb::pruned_store::exists (nano::transaction const & transaction_a, nano::block_hash const & hash_a) const
{
	return rsnano::rsn_lmdb_pruned_store_exists (handle, transaction_a.get_rust_handle (), hash_a.bytes.data ());
}

nano::block_hash nano::lmdb::pruned_store::random (nano::transaction const & transaction)
{
	nano::block_hash random_hash;
	rsnano::rsn_lmdb_pruned_store_random (handle, transaction.get_rust_handle (), random_hash.bytes.data ());
	return random_hash;
}

size_t nano::lmdb::pruned_store::count (nano::transaction const & transaction_a) const
{
	return rsnano::rsn_lmdb_pruned_store_count (handle, transaction_a.get_rust_handle ());
}

void nano::lmdb::pruned_store::clear (nano::write_transaction const & transaction_a)
{
	return rsnano::rsn_lmdb_pruned_store_clear (handle, transaction_a.get_rust_handle ());
}

nano::store_iterator<nano::block_hash, std::nullptr_t> nano::lmdb::pruned_store::begin (nano::transaction const & transaction, nano::block_hash const & hash) const
{
	auto it_handle{ rsnano::rsn_lmdb_pruned_store_begin_at_hash (handle, transaction.get_rust_handle (), hash.bytes.data ()) };
	return to_iterator (it_handle);
}

nano::store_iterator<nano::block_hash, std::nullptr_t> nano::lmdb::pruned_store::begin (nano::transaction const & transaction) const
{
	auto it_handle{ rsnano::rsn_lmdb_pruned_store_begin (handle, transaction.get_rust_handle ()) };
	return to_iterator (it_handle);
}

nano::store_iterator<nano::block_hash, std::nullptr_t> nano::lmdb::pruned_store::end () const
{
	return nano::store_iterator<nano::block_hash, std::nullptr_t> (nullptr);
}

namespace
{
void for_each_par_wrapper (void * context, rsnano::TransactionHandle * txn_handle, rsnano::LmdbIteratorHandle * begin_handle, rsnano::LmdbIteratorHandle * end_handle)
{
	auto action = static_cast<std::function<void (nano::read_transaction const &, nano::store_iterator<nano::block_hash, std::nullptr_t>, nano::store_iterator<nano::block_hash, std::nullptr_t>)> const *> (context);
	nano::read_mdb_txn txn{ txn_handle };
	auto begin{ to_iterator (begin_handle) };
	auto end{ to_iterator (end_handle) };
	(*action) (txn, std::move (begin), std::move (end));
}
void for_each_par_delete_context (void * context)
{
}
}

void nano::lmdb::pruned_store::for_each_par (std::function<void (nano::read_transaction const &, nano::store_iterator<nano::block_hash, std::nullptr_t>, nano::store_iterator<nano::block_hash, std::nullptr_t>)> const & action_a) const
{
	auto context = (void *)&action_a;
	rsnano::rsn_lmdb_pruned_store_for_each_par (handle, for_each_par_wrapper, context, for_each_par_delete_context);
}
