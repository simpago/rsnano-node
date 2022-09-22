#include <nano/node/lmdb/confirmation_height_store.hpp>
#include <nano/node/lmdb/lmdb.hpp>
#include <nano/secure/parallel_traversal.hpp>

namespace
{
nano::store_iterator<nano::account, nano::confirmation_height_info> to_iterator (rsnano::LmdbIteratorHandle * it_handle)
{
	if (it_handle == nullptr)
	{
		return nano::store_iterator<nano::account, nano::confirmation_height_info> (nullptr);
	}

	return nano::store_iterator<nano::account, nano::confirmation_height_info> (
	std::make_unique<nano::mdb_iterator<nano::account, nano::confirmation_height_info>> (it_handle));
}
}

nano::lmdb::confirmation_height_store::confirmation_height_store (rsnano::LmdbConfirmationHeightStoreHandle * handle_a) :
	handle{ handle_a }
{
}

nano::lmdb::confirmation_height_store::~confirmation_height_store ()
{
	if (handle != nullptr)
		rsnano::rsn_lmdb_confirmation_height_store_destroy (handle);
}

void nano::lmdb::confirmation_height_store::put (nano::write_transaction const & transaction, nano::account const & account, nano::confirmation_height_info const & confirmation_height_info)
{
	rsnano::rsn_lmdb_confirmation_height_store_put (handle, transaction.get_rust_handle (), account.bytes.data (), &confirmation_height_info.dto);
}

bool nano::lmdb::confirmation_height_store::get (nano::transaction const & transaction, nano::account const & account, nano::confirmation_height_info & confirmation_height_info)
{
	bool success = rsnano::rsn_lmdb_confirmation_height_store_get (handle, transaction.get_rust_handle (), account.bytes.data (), &confirmation_height_info.dto);
	return !success;
}

bool nano::lmdb::confirmation_height_store::exists (nano::transaction const & transaction, nano::account const & account) const
{
	return rsnano::rsn_lmdb_confirmation_height_store_exists (handle, transaction.get_rust_handle (), account.bytes.data ());
}

void nano::lmdb::confirmation_height_store::del (nano::write_transaction const & transaction, nano::account const & account)
{
	rsnano::rsn_lmdb_confirmation_height_store_del (handle, transaction.get_rust_handle (), account.bytes.data ());
}

uint64_t nano::lmdb::confirmation_height_store::count (nano::transaction const & transaction_a)
{
	return rsnano::rsn_lmdb_confirmation_height_store_count (handle, transaction_a.get_rust_handle ());
}

void nano::lmdb::confirmation_height_store::clear (nano::write_transaction const & transaction_a, nano::account const & account_a)
{
	del (transaction_a, account_a);
}

void nano::lmdb::confirmation_height_store::clear (nano::write_transaction const & transaction_a)
{
	rsnano::rsn_lmdb_confirmation_height_store_clear (handle, transaction_a.get_rust_handle ());
}

nano::store_iterator<nano::account, nano::confirmation_height_info> nano::lmdb::confirmation_height_store::begin (nano::transaction const & transaction, nano::account const & account) const
{
	auto it_handle{ rsnano::rsn_lmdb_confirmation_height_store_begin_at_account (handle, transaction.get_rust_handle (), account.bytes.data ()) };
	return to_iterator (it_handle);
}

nano::store_iterator<nano::account, nano::confirmation_height_info> nano::lmdb::confirmation_height_store::begin (nano::transaction const & transaction) const
{
	auto it_handle{ rsnano::rsn_lmdb_confirmation_height_store_begin (handle, transaction.get_rust_handle ()) };
	return to_iterator (it_handle);
}

nano::store_iterator<nano::account, nano::confirmation_height_info> nano::lmdb::confirmation_height_store::end () const
{
	return nano::store_iterator<nano::account, nano::confirmation_height_info> (nullptr);
}

namespace
{
void for_each_par_wrapper (void * context, rsnano::TransactionHandle * txn_handle, rsnano::LmdbIteratorHandle * begin_handle, rsnano::LmdbIteratorHandle * end_handle)
{
	auto action = static_cast<std::function<void (nano::read_transaction const &, nano::store_iterator<nano::account, nano::confirmation_height_info>, nano::store_iterator<nano::account, nano::confirmation_height_info>)> const *> (context);
	nano::read_mdb_txn txn{ txn_handle };
	auto begin{ to_iterator (begin_handle) };
	auto end{ to_iterator (end_handle) };
	(*action) (txn, std::move (begin), std::move (end));
}
void for_each_par_delete_context (void * context)
{
}
}

void nano::lmdb::confirmation_height_store::for_each_par (std::function<void (nano::read_transaction const &, nano::store_iterator<nano::account, nano::confirmation_height_info>, nano::store_iterator<nano::account, nano::confirmation_height_info>)> const & action_a) const
{
	auto context = (void *)&action_a;
	rsnano::rsn_lmdb_confirmation_height_store_for_each_par (handle, for_each_par_wrapper, context, for_each_par_delete_context);
}

MDB_dbi nano::lmdb::confirmation_height_store::table_handle () const
{
	return rsnano::rsn_lmdb_confirmation_height_store_table_handle (handle);
}