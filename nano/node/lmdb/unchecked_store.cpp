#include <nano/node/lmdb/lmdb.hpp>
#include <nano/node/lmdb/unchecked_store.hpp>
#include <nano/secure/parallel_traversal.hpp>

namespace
{
nano::store_iterator<nano::unchecked_key, nano::unchecked_info> to_iterator (rsnano::LmdbIteratorHandle * it_handle)
{
	if (it_handle == nullptr)
	{
		return { nullptr };
	}

	return { std::make_unique<nano::mdb_iterator<nano::unchecked_key, nano::unchecked_info>> (it_handle) };
}
}

nano::lmdb::unchecked_store::unchecked_store (rsnano::LmdbUncheckedStoreHandle * handle_a) :
	handle{ handle_a }
{
}

nano::lmdb::unchecked_store::~unchecked_store ()
{
	if (handle != nullptr)
		rsnano::rsn_lmdb_unchecked_store_destroy (handle);
}

void nano::lmdb::unchecked_store::clear (nano::write_transaction const & transaction_a)
{
	rsnano::rsn_lmdb_unchecked_store_clear (handle, transaction_a.get_rust_handle ());
}

void nano::lmdb::unchecked_store::put (nano::write_transaction const & transaction_a, nano::hash_or_account const & dependency, nano::unchecked_info const & info)
{
	rsnano::rsn_lmdb_unchecked_store_put (handle, transaction_a.get_rust_handle (), dependency.bytes.data (), info.handle);
}

bool nano::lmdb::unchecked_store::exists (nano::transaction const & transaction_a, nano::unchecked_key const & key)
{
	auto key_dto{ key.to_dto () };
	return rsnano::rsn_lmdb_unchecked_store_exists (handle, transaction_a.get_rust_handle (), &key_dto);
}

void nano::lmdb::unchecked_store::del (nano::write_transaction const & transaction_a, nano::unchecked_key const & key_a)
{
	auto key_dto{ key_a.to_dto () };
	rsnano::rsn_lmdb_unchecked_store_del (handle, transaction_a.get_rust_handle (), &key_dto);
}

nano::store_iterator<nano::unchecked_key, nano::unchecked_info> nano::lmdb::unchecked_store::end () const
{
	return nano::store_iterator<nano::unchecked_key, nano::unchecked_info> (nullptr);
}

nano::store_iterator<nano::unchecked_key, nano::unchecked_info> nano::lmdb::unchecked_store::begin (nano::transaction const & transaction) const
{
	auto it_handle{ rsnano::rsn_lmdb_unchecked_store_begin (handle, transaction.get_rust_handle ()) };
	return to_iterator (it_handle);
}

nano::store_iterator<nano::unchecked_key, nano::unchecked_info> nano::lmdb::unchecked_store::lower_bound (nano::transaction const & transaction, nano::unchecked_key const & key) const
{
	auto key_dto{ key.to_dto () };
	auto it_handle{ rsnano::rsn_lmdb_unchecked_store_lower_bound (handle, transaction.get_rust_handle (), &key_dto) };
	return to_iterator (it_handle);
}

size_t nano::lmdb::unchecked_store::count (nano::transaction const & transaction_a)
{
	return rsnano::rsn_lmdb_unchecked_store_count (handle, transaction_a.get_rust_handle ());
}

MDB_dbi nano::lmdb::unchecked_store::table_handle () const
{
	return rsnano::rsn_lmdb_unchecked_store_table_handle (handle);
}
