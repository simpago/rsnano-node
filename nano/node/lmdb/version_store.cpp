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

void nano::lmdb::version_store::put (nano::write_transaction const & transaction_a, int version)
{
	rsnano::rsn_lmdb_version_store_put (handle, transaction_a.get_rust_handle (), version);
}

int nano::lmdb::version_store::get (nano::transaction const & transaction_a) const
{
	return rsnano::rsn_lmdb_version_store_get (handle, transaction_a.get_rust_handle ());
}
