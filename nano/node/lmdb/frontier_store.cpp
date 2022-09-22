#include <nano/node/lmdb/frontier_store.hpp>
#include <nano/node/lmdb/lmdb.hpp>
#include <nano/secure/parallel_traversal.hpp>

nano::lmdb::frontier_store::frontier_store (rsnano::LmdbFrontierStoreHandle * handle_a) :
	handle{ handle_a }
{
}

nano::lmdb::frontier_store::~frontier_store ()
{
	if (handle != nullptr)
		rsnano::rsn_lmdb_frontier_store_destroy (handle);
}

void nano::lmdb::frontier_store::put (nano::write_transaction const & transaction, nano::block_hash const & hash, nano::account const & account)
{
	rsnano::rsn_lmdb_frontier_store_put (handle, transaction.get_rust_handle (), hash.bytes.data (), account.bytes.data ());
}

nano::account nano::lmdb::frontier_store::get (nano::transaction const & transaction, nano::block_hash const & hash) const
{
	nano::account result;
	rsnano::rsn_lmdb_frontier_store_get (handle, transaction.get_rust_handle (), hash.bytes.data (), result.bytes.data ());
	return result;
}

void nano::lmdb::frontier_store::del (nano::write_transaction const & transaction, nano::block_hash const & hash)
{
	rsnano::rsn_lmdb_frontier_store_del (handle, transaction.get_rust_handle (), hash.bytes.data ());
}

namespace
{
nano::store_iterator<nano::block_hash, nano::account> to_iterator (rsnano::LmdbIteratorHandle * it_handle)
{
	if (it_handle == nullptr)
	{
		return { nullptr };
	}

	return { std::make_unique<nano::mdb_iterator<nano::block_hash, nano::account>> (it_handle) };
}
}

nano::store_iterator<nano::block_hash, nano::account> nano::lmdb::frontier_store::begin (nano::transaction const & transaction) const
{
	auto it_handle{ rsnano::rsn_lmdb_frontier_store_begin (handle, transaction.get_rust_handle ()) };
	return to_iterator (it_handle);
}

nano::store_iterator<nano::block_hash, nano::account> nano::lmdb::frontier_store::begin (nano::transaction const & transaction, nano::block_hash const & hash) const
{
	auto it_handle{ rsnano::rsn_lmdb_frontier_store_begin_at_hash (handle, transaction.get_rust_handle (), hash.bytes.data ()) };
	return to_iterator (it_handle);
}

nano::store_iterator<nano::block_hash, nano::account> nano::lmdb::frontier_store::end () const
{
	return { nullptr };
}

namespace
{
void for_each_par_wrapper (void * context, rsnano::TransactionHandle * txn_handle, rsnano::LmdbIteratorHandle * begin_handle, rsnano::LmdbIteratorHandle * end_handle)
{
	auto action = static_cast<std::function<void (nano::read_transaction const &, nano::store_iterator<nano::block_hash, nano::account>, nano::store_iterator<nano::block_hash, nano::account>)> const *> (context);
	nano::read_mdb_txn txn{ txn_handle };
	auto begin{ to_iterator (begin_handle) };
	auto end{ to_iterator (end_handle) };
	(*action) (txn, std::move (begin), std::move (end));
}
void for_each_par_delete_context (void * context)
{
}
}

void nano::lmdb::frontier_store::for_each_par (std::function<void (nano::read_transaction const &, nano::store_iterator<nano::block_hash, nano::account>, nano::store_iterator<nano::block_hash, nano::account>)> const & action_a) const
{
	auto context = (void *)&action_a;
	rsnano::rsn_lmdb_frontier_store_for_each_par (handle, for_each_par_wrapper, context, for_each_par_delete_context);
}

MDB_dbi nano::lmdb::frontier_store::table_handle () const
{
	return rsnano::rsn_lmdb_frontier_store_table_handle (handle);
}
