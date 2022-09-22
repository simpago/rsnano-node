#include <nano/node/lmdb/lmdb.hpp>
#include <nano/node/lmdb/version_store.hpp>

nano::lmdb::version_store::version_store (rsnano::LmdbVersionStoreHandle * handle_a) :
	handle{ handle_a }
{
}

nano::lmdb::version_store::~version_store ()
{
	if (handle != nullptr)
		rsnano::rsn_lmdb_version_store_destroy (handle);
}

bool nano::lmdb::version_store::open_db (nano::transaction const & txn, uint32_t flags)
{
	return !rsnano::rsn_lmdb_version_store_open_db (handle, txn.get_rust_handle (), flags);
}

void nano::lmdb::version_store::put (nano::write_transaction const & transaction_a, int version)
{
	rsnano::rsn_lmdb_version_store_put (handle, transaction_a.get_rust_handle (), version);
}

int nano::lmdb::version_store::get (nano::transaction const & transaction_a) const
{
	return rsnano::rsn_lmdb_version_store_get (handle, transaction_a.get_rust_handle ());
}

MDB_dbi nano::lmdb::version_store::table_handle () const
{
	return rsnano::rsn_lmdb_version_store_table_handle (handle);
}
