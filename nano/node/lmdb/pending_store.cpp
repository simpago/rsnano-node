#include <nano/node/lmdb/lmdb.hpp>
#include <nano/node/lmdb/pending_store.hpp>
#include <nano/secure/parallel_traversal.hpp>

namespace
{
nano::store_iterator<nano::pending_key, nano::pending_info> to_iterator (rsnano::LmdbIteratorHandle * it_handle)
{
	if (it_handle == nullptr)
	{
		return { nullptr };
	}

	return { std::make_unique<nano::mdb_iterator<nano::pending_key, nano::pending_info>> (it_handle) };
}
}

nano::lmdb::pending_store::pending_store (rsnano::LmdbPendingStoreHandle * handle_a) :
	handle{ handle_a } {};

nano::lmdb::pending_store::~pending_store ()
{
	if (handle != nullptr)
		rsnano::rsn_lmdb_pending_store_destroy (handle);
}

namespace
{
rsnano::PendingKeyDto key_to_dto (nano::pending_key const & key)
{
	rsnano::PendingKeyDto dto;
	std::copy (std::begin (key.account.bytes), std::end (key.account.bytes), std::begin (dto.account));
	std::copy (std::begin (key.hash.bytes), std::end (key.hash.bytes), std::begin (dto.hash));
	return dto;
}

rsnano::PendingInfoDto value_to_dto (nano::pending_info const & value)
{
	rsnano::PendingInfoDto dto;
	std::copy (std::begin (value.source.bytes), std::end (value.source.bytes), std::begin (dto.source));
	std::copy (std::begin (value.amount.bytes), std::end (value.amount.bytes), std::begin (dto.amount));
	dto.epoch = static_cast<uint8_t> (value.epoch);
	return dto;
}
}

void nano::lmdb::pending_store::put (nano::write_transaction const & transaction, nano::pending_key const & key, nano::pending_info const & pending)
{
	auto key_dto{ key_to_dto (key) };
	auto value_dto{ value_to_dto (pending) };
	rsnano::rsn_lmdb_pending_store_put (handle, transaction.get_rust_handle (), &key_dto, &value_dto);
}

void nano::lmdb::pending_store::del (nano::write_transaction const & transaction, nano::pending_key const & key)
{
	auto key_dto{ key_to_dto (key) };
	rsnano::rsn_lmdb_pending_store_del (handle, transaction.get_rust_handle (), &key_dto);
}

bool nano::lmdb::pending_store::get (nano::transaction const & transaction, nano::pending_key const & key, nano::pending_info & pending_a)
{
	auto key_dto{ key_to_dto (key) };
	rsnano::PendingInfoDto value_dto;
	auto result = rsnano::rsn_lmdb_pending_store_get (handle, transaction.get_rust_handle (), &key_dto, &value_dto);
	if (!result)
	{
		std::copy (std::begin (value_dto.source), std::end (value_dto.source), std::begin (pending_a.source.bytes));
		std::copy (std::begin (value_dto.amount), std::end (value_dto.amount), std::begin (pending_a.amount.bytes));
		pending_a.epoch = static_cast<nano::epoch> (value_dto.epoch);
	}
	return result;
}

bool nano::lmdb::pending_store::exists (nano::transaction const & transaction_a, nano::pending_key const & key_a)
{
	auto key_dto{ key_to_dto (key_a) };
	return rsnano::rsn_lmdb_pending_store_exists (handle, transaction_a.get_rust_handle (), &key_dto);
}

bool nano::lmdb::pending_store::any (nano::transaction const & transaction_a, nano::account const & account_a)
{
	return rsnano::rsn_lmdb_pending_store_any (handle, transaction_a.get_rust_handle (), account_a.bytes.data ());
}

nano::store_iterator<nano::pending_key, nano::pending_info> nano::lmdb::pending_store::begin (nano::transaction const & transaction_a, nano::pending_key const & key_a) const
{
	auto key_dto{ key_to_dto (key_a) };
	auto it_handle{ rsnano::rsn_lmdb_pending_store_begin_at_key (handle, transaction_a.get_rust_handle (), &key_dto) };
	return to_iterator (it_handle);
}

nano::store_iterator<nano::pending_key, nano::pending_info> nano::lmdb::pending_store::begin (nano::transaction const & transaction_a) const
{
	auto it_handle{ rsnano::rsn_lmdb_pending_store_begin (handle, transaction_a.get_rust_handle ()) };
	return to_iterator (it_handle);
}

nano::store_iterator<nano::pending_key, nano::pending_info> nano::lmdb::pending_store::end () const
{
	return nano::store_iterator<nano::pending_key, nano::pending_info> (nullptr);
}

namespace
{
void for_each_par_wrapper (void * context, rsnano::TransactionHandle * txn_handle, rsnano::LmdbIteratorHandle * begin_handle, rsnano::LmdbIteratorHandle * end_handle)
{
	auto action = static_cast<std::function<void (nano::read_transaction const &, nano::store_iterator<nano::pending_key, nano::pending_info>, nano::store_iterator<nano::pending_key, nano::pending_info>)> const *> (context);
	nano::read_mdb_txn txn{ txn_handle };
	auto begin{ to_iterator (begin_handle) };
	auto end{ to_iterator (end_handle) };
	(*action) (txn, std::move (begin), std::move (end));
}
void for_each_par_delete_context (void * context)
{
}
}

void nano::lmdb::pending_store::for_each_par (std::function<void (nano::read_transaction const &, nano::store_iterator<nano::pending_key, nano::pending_info>, nano::store_iterator<nano::pending_key, nano::pending_info>)> const & action_a) const
{
	auto context = (void *)&action_a;
	rsnano::rsn_lmdb_pending_store_for_each_par (handle, for_each_par_wrapper, context, for_each_par_delete_context);
}

MDB_dbi nano::lmdb::pending_store::table_handle () const
{
	return rsnano::rsn_lmdb_pending_store_table_handle (handle);
}
