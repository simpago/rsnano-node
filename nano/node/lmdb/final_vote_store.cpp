#include <nano/node/lmdb/final_vote_store.hpp>
#include <nano/node/lmdb/lmdb.hpp>
#include <nano/secure/parallel_traversal.hpp>

namespace
{
nano::store_iterator<nano::qualified_root, nano::block_hash> to_iterator (rsnano::LmdbIteratorHandle * it_handle)
{
	if (it_handle == nullptr)
	{
		return nano::store_iterator<nano::qualified_root, nano::block_hash> (nullptr);
	}

	return nano::store_iterator<nano::qualified_root, nano::block_hash> (
	std::make_unique<nano::mdb_iterator<nano::qualified_root, nano::block_hash>> (it_handle));
}
}

;

nano::lmdb::final_vote_store::final_vote_store (rsnano::LmdbFinalVoteStoreHandle * handle_a) :
	handle{ handle_a }
{
}

nano::lmdb::final_vote_store::~final_vote_store ()
{
	if (handle != nullptr)
		rsnano::rsn_lmdb_final_vote_store_destroy (handle);
}

bool nano::lmdb::final_vote_store::put (nano::write_transaction const & transaction, nano::qualified_root const & root, nano::block_hash const & hash)
{
	return rsnano::rsn_lmdb_final_vote_store_put (handle, transaction.get_rust_handle (), root.bytes.data (), hash.bytes.data ());
}

std::vector<nano::block_hash> nano::lmdb::final_vote_store::get (nano::transaction const & transaction, nano::root const & root_a)
{
	rsnano::BlockHashArrayDto dto;
	rsnano::rsn_lmdb_final_vote_store_get (handle, transaction.get_rust_handle (), root_a.bytes.data (), &dto);
	std::vector<nano::block_hash> result;
	for (auto i = 0; i < dto.count / 32; ++i)
	{
		nano::block_hash hash;
		std::copy (dto.data + (i * 32), dto.data + ((i + 1) * 32), std::begin (hash.bytes));
		result.push_back (hash);
	}
	rsnano::rsn_block_hash_array_destroy (&dto);
	return result;
}

void nano::lmdb::final_vote_store::del (nano::write_transaction const & transaction, nano::root const & root)
{
	rsnano::rsn_lmdb_final_vote_store_del (handle, transaction.get_rust_handle (), root.bytes.data ());
}

size_t nano::lmdb::final_vote_store::count (nano::transaction const & transaction_a) const
{
	return rsnano::rsn_lmdb_final_vote_store_count (handle, transaction_a.get_rust_handle ());
}

void nano::lmdb::final_vote_store::clear (nano::write_transaction const & transaction_a, nano::root const & root_a)
{
	del (transaction_a, root_a);
}

void nano::lmdb::final_vote_store::clear (nano::write_transaction const & transaction_a)
{
	rsnano::rsn_lmdb_final_vote_store_clear (handle, transaction_a.get_rust_handle ());
}

nano::store_iterator<nano::qualified_root, nano::block_hash> nano::lmdb::final_vote_store::begin (nano::transaction const & transaction, nano::qualified_root const & root) const
{
	auto it_handle{ rsnano::rsn_lmdb_final_vote_store_begin_at_root (handle, transaction.get_rust_handle (), root.bytes.data ()) };
	return to_iterator (it_handle);
}

nano::store_iterator<nano::qualified_root, nano::block_hash> nano::lmdb::final_vote_store::begin (nano::transaction const & transaction) const
{
	auto it_handle{ rsnano::rsn_lmdb_final_vote_store_begin (handle, transaction.get_rust_handle ()) };
	return to_iterator (it_handle);
}

nano::store_iterator<nano::qualified_root, nano::block_hash> nano::lmdb::final_vote_store::end () const
{
	return nano::store_iterator<nano::qualified_root, nano::block_hash> (nullptr);
}

namespace
{
void for_each_par_wrapper (void * context, rsnano::TransactionHandle * txn_handle, rsnano::LmdbIteratorHandle * begin_handle, rsnano::LmdbIteratorHandle * end_handle)
{
	auto action = static_cast<std::function<void (nano::read_transaction const &, nano::store_iterator<nano::qualified_root, nano::block_hash>, nano::store_iterator<nano::qualified_root, nano::block_hash>)> const *> (context);
	nano::read_mdb_txn txn{ txn_handle };
	auto begin{ to_iterator (begin_handle) };
	auto end{ to_iterator (end_handle) };
	(*action) (txn, std::move (begin), std::move (end));
}
void for_each_par_delete_context (void * context)
{
}
}

void nano::lmdb::final_vote_store::for_each_par (std::function<void (nano::read_transaction const &, nano::store_iterator<nano::qualified_root, nano::block_hash>, nano::store_iterator<nano::qualified_root, nano::block_hash>)> const & action_a) const
{
	auto context = (void *)&action_a;
	rsnano::rsn_lmdb_final_vote_store_for_each_par (handle, for_each_par_wrapper, context, for_each_par_delete_context);
}

MDB_dbi nano::lmdb::final_vote_store::table_handle () const
{
	return rsnano::rsn_lmdb_final_vote_store_table_handle (handle);
}
