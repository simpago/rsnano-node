#include <nano/node/lmdb/account_store.hpp>
#include <nano/node/lmdb/lmdb.hpp>
#include <nano/secure/parallel_traversal.hpp>

nano::lmdb::account_store::account_store (rsnano::LmdbAccountStoreHandle * handle_a) :
	handle{ handle_a }
{
}

nano::lmdb::account_store::~account_store ()
{
	if (handle != nullptr)
		rsnano::rsn_lmdb_account_store_destroy (handle);
}

void nano::lmdb::account_store::put (nano::write_transaction const & transaction, nano::account const & account, nano::account_info const & info)
{
	rsnano::rsn_lmdb_account_store_put (handle, transaction.get_rust_handle (), account.bytes.data (), info.handle);
}

bool nano::lmdb::account_store::get (nano::transaction const & transaction, nano::account const & account, nano::account_info & info)
{
	bool found = rsnano::rsn_lmdb_account_store_get (handle, transaction.get_rust_handle (), account.bytes.data (), info.handle);
	return !found;
}

void nano::lmdb::account_store::del (nano::write_transaction const & transaction_a, nano::account const & account_a)
{
	rsnano::rsn_lmdb_account_store_del (handle, transaction_a.get_rust_handle (), account_a.bytes.data ());
}

bool nano::lmdb::account_store::exists (nano::transaction const & transaction_a, nano::account const & account_a)
{
	auto iterator (begin (transaction_a, account_a));
	return iterator != end () && nano::account (iterator->first) == account_a;
}

size_t nano::lmdb::account_store::count (nano::transaction const & transaction_a)
{
	return rsnano::rsn_lmdb_account_store_count (handle, transaction_a.get_rust_handle ());
}

nano::store_iterator<nano::account, nano::account_info> to_account_iterator (rsnano::LmdbIteratorHandle * it_handle)
{
	if (it_handle == nullptr)
	{
		return nano::store_iterator<nano::account, nano::account_info> (nullptr);
	}

	return nano::store_iterator<nano::account, nano::account_info> (
	std::make_unique<nano::mdb_iterator<nano::account, nano::account_info>> (it_handle));
}

nano::store_iterator<nano::account, nano::account_info> nano::lmdb::account_store::begin (nano::transaction const & transaction, nano::account const & account) const
{
	auto it_handle{ rsnano::rsn_lmdb_account_store_begin_account (handle, transaction.get_rust_handle (), account.bytes.data ()) };
	return to_account_iterator (it_handle);
}

nano::store_iterator<nano::account, nano::account_info> nano::lmdb::account_store::begin (nano::transaction const & transaction) const
{
	auto it_handle{ rsnano::rsn_lmdb_account_store_begin (handle, transaction.get_rust_handle ()) };
	return to_account_iterator (it_handle);
}

nano::store_iterator<nano::account, nano::account_info> nano::lmdb::account_store::rbegin (nano::transaction const & transaction_a) const
{
	auto it_handle{ rsnano::rsn_lmdb_account_store_rbegin (handle, transaction_a.get_rust_handle ()) };
	return to_account_iterator (it_handle);
}

nano::store_iterator<nano::account, nano::account_info> nano::lmdb::account_store::end () const
{
	return nano::store_iterator<nano::account, nano::account_info> (nullptr);
}

namespace
{
void for_each_par_wrapper (void * context, rsnano::TransactionHandle * txn_handle, rsnano::LmdbIteratorHandle * begin_handle, rsnano::LmdbIteratorHandle * end_handle)
{
	auto action = static_cast<std::function<void (nano::read_transaction const &, nano::store_iterator<nano::account, nano::account_info>, nano::store_iterator<nano::account, nano::account_info>)> const *> (context);
	nano::read_mdb_txn txn{ txn_handle };
	auto begin{ to_account_iterator (begin_handle) };
	auto end{ to_account_iterator (end_handle) };
	(*action) (txn, std::move (begin), std::move (end));
}
void for_each_par_delete_context (void * context)
{
}
}

void nano::lmdb::account_store::for_each_par (std::function<void (nano::read_transaction const &, nano::store_iterator<nano::account, nano::account_info>, nano::store_iterator<nano::account, nano::account_info>)> const & action_a) const
{
	auto context = (void *)&action_a;
	rsnano::rsn_lmdb_account_store_for_each_par (handle, for_each_par_wrapper, context, for_each_par_delete_context);
}

MDB_dbi nano::lmdb::account_store::get_accounts_handle () const
{
	return rsnano::rsn_lmdb_account_store_accounts_handle (handle);
}
